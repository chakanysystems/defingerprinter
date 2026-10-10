(defpackage #:defingerprinter-tests
  (:use #:cl)
  (:export #:run-tests))
(in-package #:defingerprinter-tests)

(defun check (truth)
  (unless truth (error "Defingerprinter CFFI test failed.")))

(defun check-status (context text expected &key (language "en"))
  (multiple-value-bind (safe status)
      (defingerprinter:protect context text :language language)
    (check (eq status expected))
    (unless (eq status :ok) (check (null safe)))
    safe))

(defun context-options ()
  (let ((model (uiop:getenv "RAMPART_MODEL_DIR"))
        (runtime (uiop:getenv "RAMPART_RUNTIME")))
    (when (or model runtime)
      (check (and model runtime))
      (list :model-directory model :runtime-library runtime))))

(defvar *fail-context-construction* nil)
(defvar *failed-native-handle* nil)

(defmethod initialize-instance :before ((context defingerprinter:redactor)
                                        &key handle &allow-other-keys)
  (declare (ignore context))
  (when *fail-context-construction*
    (setf *failed-native-handle* handle)
    (error "Synthetic context construction failure.")))

(defun run-tests ()
  "Run actual C ABI inference. DEFINGERPRINTER_LIBRARY is required.
RAMPART_MODEL_DIR and RAMPART_RUNTIME select explicit raw-Cargo artifacts;
omit both to exercise the compiled Nix defaults."
  (let ((library (uiop:getenv "DEFINGERPRINTER_LIBRARY")))
    (unless library (error "Set DEFINGERPRINTER_LIBRARY for CFFI tests."))
    (defingerprinter:load-library library))
  (let ((options (context-options)))
    ;; A Lisp constructor failure must release the already-created native handle.
    (let ((*fail-context-construction* t)
          (*failed-native-handle* nil))
      (check (handler-case (progn (apply #'defingerprinter:make-redactor options) nil)
               (defingerprinter:setup-error (condition)
                 (eq :artifact-error (defingerprinter:setup-error-status condition)))))
      (check *failed-native-handle*)
      (check (= 2 (cffi:foreign-funcall-pointer
                   (defingerprinter::native-symbol "defingerprinter_destroy") ()
                   :uint64 *failed-native-handle* :int32))))
    ;; A new context accepts protection immediately, without eager model setup.
    (let ((context (apply #'defingerprinter:make-redactor options)))
      (unwind-protect
           (progn
             (check-status context '(not a string) :invalid-argument :language 42)
             (let ((safe (check-status context "Email alice@example.com." :ok)))
               (check (stringp safe))
               (check (search "[EMAIL_1]" safe))
               (check (not (search "alice@example.com" safe))))
             (let ((safe (check-status context "Café: alice@example.com" :ok)))
               (check (search "[EMAIL_1]" safe)))
             (check (string= "" (check-status context "" :ok)))
             (check-status context "hello" :unsupported-input :language "zz")
             (check-status context (make-string 16385 :initial-element #\a) :unsupported-input)
             (check-status context 42 :invalid-argument)
             ;; Concurrent protect/close serializes through output decoding too.
             (let ((thread (bt:make-thread
                            (lambda ()
                              (dotimes (index 10)
                                (declare (ignorable index))
                                (multiple-value-bind (safe status)
                                    (defingerprinter:protect context "alice@example.com")
                                  (check (member status '(:ok :invalid-handle)))
                                  (when (eq status :invalid-handle) (check (null safe)))))))))
               (check (eq :ok (defingerprinter:close-redactor context)))
               (bt:join-thread thread))
             (check-status context '(not a string) :invalid-handle :language 42))
        (check (eq :ok (defingerprinter:close-redactor context))))
      (check-status context nil :invalid-handle)
      (check (eq :ok (defingerprinter:close-redactor context))))
    ;; UNWIND-PROTECT in WITH-REDACTOR also handles nonlocal exits.
    (let ((context nil))
      (catch 'finished
        (defingerprinter:with-redactor (instance
                                       :model-directory (getf options :model-directory)
                                       :runtime-library (getf options :runtime-library))
          (setf context instance)
          (throw 'finished t)))
      (check-status context nil :invalid-handle)))
  (format t "Defingerprinter CFFI tests passed.~%")
  t)
