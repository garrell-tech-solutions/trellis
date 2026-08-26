#!/usr/bin/env bb
;; Project-specific acceptance-entrypoint-generator (APS acceptance-generator.md).
;; Turns one parser-produced JSON IR file into a Rust integration test under
;; crates/acceptance-tests/tests/, plus a metadata file recording which
;; generated files it produced and their implementation hash.
;;
;; Usage: generate.bb <json-ir> <generated-test-output-dir>

(ns generate
  (:require [cheshire.core :as json]
            [clojure.string :as str]
            [babashka.fs :as fs]
            [babashka.process :as process])
  (:import [java.security MessageDigest]))

(defn usage! []
  (binding [*out* *err*]
    (println "usage: generate.bb <json-ir> <generated-test-output-dir>"))
  (System/exit 2))

(defn sha256-hex [^String s]
  (let [md (MessageDigest/getInstance "SHA-256")
        digest (.digest md (.getBytes s "UTF-8"))]
    (apply str (map #(format "%02x" (bit-and % 0xff)) digest))))

;; Feature path is reconstructed by convention: this pipeline always names an
;; IR file build/acceptance/ir/<slug>.json for features/<slug>.feature (see
;; scripts/acceptance/run.sh), so the generator does not need the feature path
;; as a separate argument.
(defn slug-from-ir-path [ir-path]
  (str/replace (fs/file-name ir-path) #"\.json$" ""))

(defn metadata-name [feature-path]
  (-> feature-path
      str/lower-case
      (str/replace #"[^a-z0-9]+" "-")
      (str/replace #"^-+|-+$" "")
      (str ".json")))

(defn write-if-changed
  "Write `contents` to `path` only when that would change the file, and say
  whether it did. The point is the mtime: an unchanged file that is rewritten
  anyway is what makes Cargo rebuild a test binary whose source did not move."
  [path contents]
  (if (and (fs/exists? path) (= contents (slurp (str path))))
    false
    (do (spit (str path) contents) true)))

(defn generated-test-source [slug feature-path rel-ir fn-name]
  (str
    "// GENERATED FILE. Do not edit.\n"
    "// Source feature: " feature-path "\n"
    "// Regenerate with scripts/acceptance/run.sh\n"
    "\n"
    "const IR_JSON: &str = include_str!(\"" rel-ir "\");\n"
    "\n"
    "#[tokio::test]\n"
    "async fn " fn-name "() {\n"
    "    let feature: acceptance_tests::ir::Feature =\n"
    "        serde_json::from_str(IR_JSON).expect(\"parse generated JSON IR\");\n"
    "    let results = acceptance_tests::runtime::run_feature(&feature).await;\n"
    "    let failures: Vec<String> = results\n"
    "        .into_iter()\n"
    "        .filter_map(|(name, outcome)| outcome.err().map(|e| format!(\"{name}: {e}\")))\n"
    "        .collect();\n"
    "    assert!(failures.is_empty(), \"acceptance failures for " slug ":\\n{}\", failures.join(\"\\n\"));\n"
    "}\n"))

(defn -main [& args]
  (when (not= 2 (count args))
    (usage!))
  (let [[ir-path out-dir] args]
    (when-not (fs/exists? ir-path)
      (binding [*out* *err*] (println "IR file not found:" ir-path))
      (System/exit 1))
    (let [slug (slug-from-ir-path ir-path)
          feature-path (str "features/" slug ".feature")
          fn-name (str slug "_acceptance")
          out-dir-abs (fs/absolutize out-dir)
          test-file (fs/path out-dir-abs (str slug "_acceptance.rs"))
          metadata-dir (fs/path out-dir-abs "metadata")
          metadata-file (fs/path metadata-dir (metadata-name feature-path))
          ir-abs (fs/absolutize ir-path)
          rel-ir (str (fs/relativize out-dir-abs ir-abs))
          contents (generated-test-source slug feature-path rel-ir fn-name)]
      (fs/create-dirs out-dir-abs)
      (fs/create-dirs metadata-dir)
      ;; Format and compare in a scratch file, never in place.
      ;;
      ;; Every one of these outputs is an input to `cargo test`: the entry
      ;; point is compiled, and it `include_str!`s the IR beside it. Cargo
      ;; decides what to rebuild from mtimes, so writing a byte-identical file
      ;; is not free -- it is a full rebuild of all of them. Regeneration is
      ;; deterministic and the tree usually has one changed feature in it or
      ;; none, so the normal case is that nothing here has moved and the
      ;; cheapest correct thing to do is leave the files alone.
      ;;
      ;; Formatting has to happen before the comparison rather than after,
      ;; because the bytes that land on disk are rustfmt's, not this
      ;; generator's, and comparing the two would report every file as changed
      ;; forever. The project has no rustfmt.toml, and --edition is passed
      ;; explicitly, so formatting a scratch file outside the tree gives the
      ;; same output as formatting one inside it.
      (let [scratch (fs/create-temp-file {:prefix "aps-acceptance-entrypoint"
                                          :suffix ".rs"})]
        (try
          (spit (str scratch) contents)
          (let [{:keys [exit err]} (process/sh "rustfmt" "--edition" "2021" (str scratch))]
            (when-not (zero? exit)
              (binding [*out* *err*] (println "rustfmt failed on" (str test-file) ":" err))
              (System/exit 1)))
          (let [formatted (slurp (str scratch))
                rel-generated (str (fs/relativize out-dir-abs test-file))
                metadata {:schema_version 1
                           :feature_path feature-path
                           :ir_path (str ir-path)
                           :implementation_hash (str "sha256:" (sha256-hex formatted))
                           :hash_scope "generated_files"
                           :generated_files [rel-generated]}
                metadata-json (json/generate-string metadata {:pretty true})
                test-changed (write-if-changed test-file formatted)
                metadata-changed (write-if-changed metadata-file metadata-json)]
            (println (if (or test-changed metadata-changed) "generated:" "unchanged:")
                     (str test-file)))
          (finally (fs/delete-if-exists scratch)))))))

(apply -main *command-line-args*)
