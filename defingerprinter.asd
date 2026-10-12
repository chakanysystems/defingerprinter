(asdf:defsystem "defingerprinter"
  :description "Chakany Systems Defingerprinter C ABI bindings"
  :version "0.1.0"
  :depends-on ("cffi" "babel" "bordeaux-threads")
  :serial t
  :components ((:file "lisp/bindings"))
  :in-order-to ((asdf:test-op (asdf:test-op "defingerprinter/tests"))))

(asdf:defsystem "defingerprinter/tests"
  :depends-on ("defingerprinter")
  :components ((:file "tests/cffi"))
  :perform (asdf:test-op (operation component)
             (declare (ignore operation component))
             (uiop:symbol-call :defingerprinter-tests :run-tests)))
