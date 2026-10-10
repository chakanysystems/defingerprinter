(defpackage #:defingerprinter
  (:use #:cl)
  (:export #:load-library #:make-redactor #:protect
           #:close-redactor #:with-redactor #:redactor
           #:setup-error #:setup-error-status))
(in-package #:defingerprinter)

(define-condition setup-error (error)
  ((status :initarg :status :reader setup-error-status))
  (:report (lambda (condition stream)
             (declare (ignore condition))
             (write-string "Defingerprinter setup failed." stream))))

(defparameter *library-lock* (bt:make-lock "Defingerprinter library"))
(defvar *library* nil)
(defvar *library-path* nil)

(defun status-keyword (code)
  (case code
    (0 :ok) (1 :invalid-argument) (2 :invalid-handle)
    (4 :unsupported-input) (5 :artifact-error) (6 :runtime-error)
    (7 :inference-error) (8 :buffer-too-small) (otherwise :internal-error)))

(defun setup-failure (status)
  (error 'setup-error :status status))

(defun absolute-path-string (path)
  (unless (and (or (pathnamep path) (stringp path))
               (uiop:absolute-pathname-p path))
    (setup-failure :invalid-argument))
  (let ((name (namestring path)))
    (when (find #\Null name) (setup-failure :invalid-argument))
    name))

(defun native-symbol (name)
  (cffi:foreign-symbol-pointer name :library *library*))

(defun load-library (path)
  "Load one absolute shared-library path, pinned for the lifetime of this image.
A RETRY restart permits correcting a failed setup without retaining its cause."
  (loop
    (restart-case
        (return
          (handler-case
              (bt:with-lock-held (*library-lock*)
                (let ((name (absolute-path-string path)))
                  (when (and *library-path* (not (equal name *library-path*)))
                    (setup-failure :invalid-argument))
                  (let ((library (or *library* (cffi:load-foreign-library name)))
                        (committed nil))
                    (unwind-protect
                         (progn
                           (unless (= 2 (cffi:foreign-funcall-pointer
                                         (cffi:foreign-symbol-pointer
                                          "defingerprinter_abi_version" :library library)
                                         () :uint32))
                             (setup-failure :unsupported-input))
                           (setf *library* library *library-path* name committed t)
                           t)
                      ;; Rejected libraries have no contexts and may be unloaded.
                      (unless (or committed *library*)
                        (cffi:close-foreign-library library))))))
            (setup-error (condition) (error condition))
            (error () (setup-failure :artifact-error))))
      (retry () :report "Retry Defingerprinter library setup."))))

(defclass redactor ()
  ((handle :initarg :handle :accessor redactor-handle)
   (lock :initform (bt:make-lock "Defingerprinter context") :reader redactor-lock)))

(defun path-octets (path)
  (let ((bytes (babel:string-to-octets (absolute-path-string path) :encoding :utf-8)))
    (unless (<= 1 (length bytes) 4096) (setup-failure :invalid-argument))
    bytes))

(defun create-native (model runtime)
  (cffi:with-foreign-object (handle :uint64)
    (setf (cffi:mem-ref handle :uint64) 0)
    (labels ((create (model-pointer model-length runtime-pointer runtime-length)
               (let ((status
                       (status-keyword
                        (cffi:foreign-funcall-pointer
                         (native-symbol "defingerprinter_create") ()
                         :pointer model-pointer :size model-length
                         :pointer runtime-pointer :size runtime-length
                         :pointer handle :int32))))
                 (unless (eq status :ok) (setup-failure status))
                (let ((context nil)
                      (native-handle (cffi:mem-ref handle :uint64)))
                  (unwind-protect
                       (setf context (make-instance 'redactor :handle native-handle))
                    (unless context
                      (cffi:foreign-funcall-pointer
                       (native-symbol "defingerprinter_destroy") ()
                       :uint64 native-handle :int32)))))))
      (if model
          (cffi:with-pointer-to-vector-data (model-pointer model)
            (cffi:with-pointer-to-vector-data (runtime-pointer runtime)
              (create model-pointer (length model) runtime-pointer (length runtime))))
          (create (cffi:null-pointer) 0 (cffi:null-pointer) 0)))))

(defun make-redactor (&key model-directory runtime-library)
  "Create a live context. Omit both paths for immutable Nix defaults.
Explicit paths must be absolute; model initialization is lazy."
  (loop
    (restart-case
        (return
          (handler-case
              (bt:with-lock-held (*library-lock*)
                (unless *library* (setup-failure :artifact-error))
                (unless (eq (null model-directory) (null runtime-library))
                  (setup-failure :invalid-argument))
                (create-native (and model-directory (path-octets model-directory))
                               (and runtime-library (path-octets runtime-library))))
            (setup-error (condition) (error condition))
            (error () (setup-failure :artifact-error))))
      (retry () :report "Retry Defingerprinter context setup."))))

(defun protect-native (context text language)
  (unless (and (stringp text) (stringp language))
    (return-from protect-native (values nil :invalid-argument)))
  (unless (and (<= (length text) 16384)
               (member language '("en" "es" "fr" "de" "it" "pt" "nl") :test #'string=))
    (return-from protect-native (values nil :unsupported-input)))
  (let ((input (babel:string-to-octets text :encoding :utf-8))
        (language-bytes (babel:string-to-octets language :encoding :utf-8)))
    (when (> (length input) 16384)
      (return-from protect-native (values nil :unsupported-input)))
    (cffi:with-pointer-to-vector-data (input-pointer input)
      (cffi:with-pointer-to-vector-data (language-pointer language-bytes)
        (cffi:with-foreign-objects ((output :uint8 4096) (written :size))
          (setf (cffi:mem-ref written :size) 0)
          (let ((status
                  (status-keyword
                   (cffi:foreign-funcall-pointer
                    (native-symbol "defingerprinter_protect") ()
                    :uint64 (redactor-handle context)
                    :pointer language-pointer :size (length language-bytes)
                    :pointer input-pointer :size (length input)
                    :pointer output :size 4096 :pointer written :int32))))
            (unless (eq status :ok) (return-from protect-native (values nil status)))
            (let ((length (cffi:mem-ref written :size)))
              (unless (<= length 4096)
                (return-from protect-native (values nil :internal-error)))
              (let ((bytes (make-array length :element-type '(unsigned-byte 8))))
                (dotimes (index length)
                  (setf (aref bytes index) (cffi:mem-aref output :uint8 index)))
                (values (babel:octets-to-string bytes :encoding :utf-8 :errorp t) :ok)))))))))

(defun protect (context text &key (language "en"))
  "Return the safe string and :OK, or NIL and a failure status.
The context lock covers inference and output decoding. No raw-text conditions escape."
  (bt:with-lock-held ((redactor-lock context))
    (unless (redactor-handle context) (return-from protect (values nil :invalid-handle)))
    (handler-case (protect-native context text language)
      (error () (values nil :internal-error)))))

(defun close-redactor (context)
  "Destroy a context synchronously. Repeated closes return :OK."
  (bt:with-lock-held ((redactor-lock context))
    (if (redactor-handle context)
        (let ((status (status-keyword
                       (cffi:foreign-funcall-pointer
                        (native-symbol "defingerprinter_destroy") ()
                        :uint64 (redactor-handle context) :int32))))
          (when (member status '(:ok :invalid-handle))
            (setf (redactor-handle context) nil))
          status)
        :ok)))

(defmacro with-redactor ((variable &rest options) &body body)
  "Create a live context and close it on every exit."
  `(let ((,variable (make-redactor ,@options)))
     (unwind-protect (progn ,@body)
       (close-redactor ,variable))))
