//! Profile the pinned graph and compare exact-length input specialization.
#[path = "../src/tract_weights.rs"]
mod tract_weights;
use defingerprinter::model::ARTIFACTS;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, hint::black_box, path::PathBuf, time::Instant};
use tract_onnx::{
    prelude::*,
    tract_core::{internal::ensure, ops::matmul::optimized::OptMatMul, plan::eval},
};

#[derive(Clone, Copy, Debug)]
enum Mode {
    Dynamic,
    Expanded,
    ExpandedBatchOne,
    ExpandedFixed,
    BatchOne,
    Fixed,
    Constants,
}

fn inputs(ids: &[i64], mode: Mode) -> TractResult<TVec<TValue>> {
    let length = ids.len();
    let mut inputs = tvec![Tensor::from_shape(&[1, length], ids)?.into()];
    if !matches!(mode, Mode::Constants) {
        inputs.push(Tensor::from_shape(&[1, length], &vec![1i64; length])?.into());
        inputs.push(Tensor::from_shape(&[1, length], &vec![0i64; length])?.into());
    }
    Ok(inputs)
}

fn load(bytes: &[u8], length: usize, mode: Mode) -> TractResult<TypedSimpleState> {
    let mut onnx = tract_onnx::onnx();
    // Internal ONNX value_info repeats the symbolic input dimensions. Infer these
    // shapes again when fixing the inputs, rather than mixing symbols and values.
    onnx.ignore_value_info = !matches!(mode, Mode::Dynamic | Mode::Expanded);
    let mut model = onnx.model_for_read(&mut std::io::Cursor::new(bytes))?;
    let names: Vec<_> = model
        .input_outlets()?
        .iter()
        .map(|outlet| model.node(outlet.node).name.as_str())
        .collect();
    ensure!(
        names == ["input_ids", "attention_mask", "token_type_ids"],
        "unexpected input order"
    );
    if !matches!(mode, Mode::Dynamic | Mode::Expanded) {
        let sequence = if matches!(mode, Mode::BatchOne | Mode::ExpandedBatchOne) {
            model.sym("sequence_length").to_dim()
        } else {
            length.to_dim()
        };
        for input in 0..3 {
            model.set_input_fact(
                input,
                InferenceFact::dt_shape(i64::datum_type(), [1.to_dim(), sequence.clone()]),
            )?;
        }
    }
    if matches!(mode, Mode::Constants) {
        for (input, value) in [(1, 1i64), (2, 0i64)] {
            let tensor = Tensor::from_shape(&[1, length], &vec![value; length])?;
            model.set_input_fact(input, tensor.clone().into())?;
            let outlet = model.input_outlets()?[input];
            model.node_mut(outlet.node).op = Box::new(
                tract_onnx::tract_core::ops::konst::Const::new(tensor.into_arc_tensor())?,
            );
        }
        let input = model.input_outlets()?[0];
        model.set_input_outlets(&[input])?;
    }
    let mut model = model.into_typed()?;
    if matches!(
        mode,
        Mode::Expanded | Mode::ExpandedBatchOne | Mode::ExpandedFixed
    ) {
        let (packed, expanded) = tract_weights::expand_weights(&mut model)?;
        println!("Expanded weights: packed_bytes={packed} expanded_bytes={expanded}");
    }
    model.into_optimized()?.into_runnable()?.spawn()
}

fn logits(state: &mut TypedSimpleState, ids: &[i64], mode: Mode) -> TractResult<Vec<f32>> {
    let output = state.run(inputs(ids, mode)?)?;
    ensure!(
        output.len() == 1 && output[0].shape() == [1, ids.len(), 35],
        "unexpected output shape"
    );
    Ok(output[0].try_as_plain_ram()?.as_slice::<f32>()?.to_vec())
}

fn verify_logits(actual: &[f32], expected: &[f32]) -> TractResult<f32> {
    ensure!(actual.len() == expected.len(), "logit count changed");
    ensure!(actual.iter().all(|v| v.is_finite()), "non-finite logits");
    let error = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0f32, f32::max);
    ensure!(error <= 0.0001, "specialization changed logits: {error}");
    let label = |row: &[f32]| {
        row.iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0
    };
    ensure!(
        actual
            .as_chunks::<35>()
            .0
            .iter()
            .zip(expected.as_chunks::<35>().0.iter())
            .all(|(a, b)| label(a) == label(b)),
        "specialization changed token labels"
    );
    Ok(error)
}

fn main() -> TractResult<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        (1..=4).contains(&args.len()),
        "usage: tract_profile MODEL_DIR [LENGTH [RUNS [all|weights|weights-reverse|shapes|packed|expanded]]]"
    );
    let directory = PathBuf::from(&args[0]);
    let length = args
        .get(1)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(128usize);
    let runs = args
        .get(2)
        .map(|s| s.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(15usize);
    ensure!(
        (2..=512).contains(&length) && runs > 0,
        "invalid length or run count"
    );
    let selection = args.get(3).map(|s| s.to_string_lossy());
    let modes: &[Mode] = match selection.as_deref().unwrap_or("all") {
        "all" => &[
            Mode::Dynamic,
            Mode::Expanded,
            Mode::ExpandedBatchOne,
            Mode::ExpandedFixed,
            Mode::BatchOne,
            Mode::Fixed,
            Mode::Constants,
        ],
        "weights" => &[Mode::Dynamic, Mode::Expanded],
        "weights-reverse" => &[Mode::Expanded, Mode::Dynamic],
        "shapes" => &[Mode::Expanded, Mode::ExpandedBatchOne, Mode::ExpandedFixed],
        "packed" => &[Mode::Dynamic],
        "expanded" => &[Mode::Expanded],
        _ => {
            return Err(tract_onnx::tract_core::internal::format_err!(
                "unknown mode selection"
            ));
        }
    };
    let mut bytes = Vec::new();
    for (name, hash) in ARTIFACTS {
        let artifact = fs::read(directory.join(name))?;
        ensure!(
            format!("{:x}", Sha256::digest(&artifact)) == hash,
            "artifact hash mismatch: {name}"
        );
        if name.ends_with(".onnx") {
            bytes = artifact;
        }
    }
    let proto = tract_onnx::onnx().proto_model_for_read(&mut std::io::Cursor::new(&bytes))?;
    let mut quantization = BTreeMap::new();
    for node in &proto.graph.as_ref().unwrap().node {
        if node.op_type == "MatMulNBits" {
            let attribute =
                |name: &str| node.attribute.iter().find(|a| a.name == name).map(|a| a.i);
            *quantization
                .entry((attribute("bits"), attribute("block_size")))
                .or_insert(0usize) += 1;
        }
    }
    println!("MatMulNBits counts by (bits, block_size): {quantization:?}");
    let tokenizer = defingerprinter::tokenizer::Tokenizer::from_json(&fs::read(
        directory.join("tokenizer.json"),
    )?)?;
    let mut ids = vec![tokenizer.tokenize("a")[0].id; length];
    ids[0] = 2;
    ids[length - 1] = 3;
    #[derive(serde::Deserialize)]
    struct Fixture {
        text: String,
    }
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!("tract_cases.json"))?;
    let mut samples = vec![ids.clone()];
    for fixture in fixtures {
        let mut sample = vec![2];
        sample.extend(
            tokenizer
                .tokenize(&fixture.text)
                .iter()
                .take(length - 2)
                .map(|t| t.id),
        );
        sample.resize(length - 1, tokenizer.tokenize("a")[0].id);
        sample.push(3);
        samples.push(sample);
    }
    // Extend or truncate each fixture to the exact length without masked padding.
    let mut reference: Option<Vec<Vec<f32>>> = None;
    for &mode in modes {
        let start = Instant::now();
        let mut state = load(&bytes, length, mode)?;
        let load_ms = start.elapsed().as_secs_f64() * 1000.0;
        let actual: Vec<_> = samples
            .iter()
            .map(|sample| logits(&mut state, sample, mode))
            .collect::<TractResult<_>>()?;
        if let Some(expected) = &reference {
            let mut error = 0f32;
            for (a, b) in actual.iter().zip(expected) {
                error = error.max(verify_logits(a, b)?);
            }
            println!(
                "{mode:?} verification_cases={} max_logit_error={error:.8}, labels equal",
                samples.len()
            );
        } else {
            ensure!(
                actual.iter().flatten().all(|v| v.is_finite()),
                "non-finite reference logits"
            );
            reference = Some(actual);
        }
        for _ in 0..3 {
            black_box(logits(&mut state, &ids, mode)?);
        }
        let mut times = Vec::new();
        for _ in 0..runs {
            let start = Instant::now();
            black_box(logits(&mut state, &ids, mode)?);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{mode:?} tokens={length} load_ms={load_ms:.3} median_ms={:.3} nodes={}",
            times[times.len() / 2],
            state.plan().order_without_consts().len()
        );
        let mut kernels = BTreeMap::new();
        for &node in state.plan().order_without_consts() {
            if let Some(matmul) = state.plan().model().node(node).op_as::<OptMatMul>() {
                *kernels.entry(format!("{:?}", matmul.mmm)).or_insert(0usize) += 1;
            }
        }
        println!("{mode:?} matrix kernel configurations: {kernels:?}");
        // Profile separately so clock reads do not affect the latency above.
        let mut by_op: BTreeMap<String, f64> = BTreeMap::new();
        let mut by_node: BTreeMap<String, f64> = BTreeMap::new();
        for _ in 0..runs {
            black_box(state.run_plan_with_eval(
                inputs(&ids, mode)?,
                |ctx, op_state, node, input| {
                    let start = Instant::now();
                    let result = eval(ctx, op_state, node, input);
                    let ms = start.elapsed().as_secs_f64() * 1000.0 / runs as f64;
                    *by_op.entry(node.op().name().into_owned()).or_default() += ms;
                    *by_node
                        .entry(format!("{} ({})", node.name, node.op().name()))
                        .or_default() += ms;
                    result
                },
            )?);
        }
        for (heading, entries) in [("operators", by_op), ("nodes", by_node)] {
            let mut entries: Vec<_> = entries.into_iter().collect();
            entries.sort_by(|a, b| b.1.total_cmp(&a.1));
            println!("{mode:?} top {heading} (mean ms per inference):");
            for (name, ms) in entries.into_iter().take(12) {
                println!("  {ms:.3} {name}");
            }
        }
    }
    Ok(())
}
