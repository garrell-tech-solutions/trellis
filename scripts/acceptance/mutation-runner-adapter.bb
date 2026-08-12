#!/usr/bin/env bb
;; Project-specific runner adapter for `gherkin-mutator --runner-worker`
;; (mutator-spec.md "Runner Adapter"). Hides that this project runs
;; acceptance tests with `cargo test`.
;;
;; Usage: mutation-runner-adapter.bb <feature-slug>
;;
;; Persistent worker: reads one newline-delimited JSON job request per line
;; from stdin, swaps the content of the compiled-in IR file
;; (build/acceptance/ir/<feature-slug>.json, embedded via `include_str!` in
;; the generated test at compile time -- cargo tracks that as a rebuild
;; dependency, so rewriting it and re-running `cargo test` picks it up) for
;; each mutation, runs the matching generated acceptance test, and writes
;; one newline-delimited JSON job response per line to stdout. All
;; diagnostics go to stderr; stdout carries only job responses.

(ns mutation-runner-adapter
  (:require [cheshire.core :as json]
            [clojure.string :as str]
            [babashka.fs :as fs]
            [babashka.process :as p]))

(defn usage! []
  (binding [*out* *err*] (println "usage: mutation-runner-adapter.bb <feature-slug>"))
  (System/exit 2))

(defn metadata-for-slug [slug]
  (let [dir (fs/path "crates/acceptance-tests/tests/metadata")
        candidates (->> (fs/list-dir dir)
                        (filter #(str/ends-with? (str %) ".json")))
        matches (->> candidates
                     (map #(json/parse-string (slurp (str %)) true))
                     (filter #(str/ends-with? (:feature_path %) (str slug ".feature"))))]
    (or (first matches)
        (do (binding [*out* *err*]
              (println "no metadata found for feature slug" slug "under" (str dir)))
            (System/exit 2)))))

(defn parse-timeout-seconds [s]
  (cond
    (str/blank? s) 60
    (str/ends-with? s "ms") (max 1 (quot (Long/parseLong (subs s 0 (- (count s) 2))) 1000))
    (str/ends-with? s "s") (Long/parseLong (subs s 0 (dec (count s))))
    (str/ends-with? s "m") (* 60 (Long/parseLong (subs s 0 (dec (count s)))))
    :else (Long/parseLong s)))

(defn run-job [ir-path test-name job]
  (let [id (get job "id")
        feature-json (get job "feature_json")
        timeout-s (parse-timeout-seconds (get job "timeout" ""))
        start-ns (System/nanoTime)]
    (spit ir-path (slurp feature-json))
    (let [{:keys [exit out err]}
          (p/shell {:out :string :err :string :continue true}
                   "timeout" (str timeout-s "s")
                   "cargo" "test" "-p" "acceptance-tests" "--test" test-name)
          duration-ns (- (System/nanoTime) start-ns)
          combined (str out err)
          timed-out? (= exit 124)
          outcome (cond
                    (zero? exit) "test_success"
                    timed-out? "infrastructure_error"
                    (re-find #"test result: FAILED|panicked at" combined) "test_failure"
                    :else "infrastructure_error")]
      {:id id
       :outcome outcome
       :output combined
       :error (if (= outcome "infrastructure_error")
                (if timed-out? (str "timed out after " timeout-s "s") combined)
                "")
       :duration duration-ns})))

(defn -main [& args]
  (when (not= 1 (count args))
    (usage!))
  (let [slug (first args)
        metadata (metadata-for-slug slug)
        ir-path (:ir_path metadata)
        test-name (-> (:generated_files metadata) first (str/replace #"\.rs$" ""))
        original-ir (slurp ir-path)]
    (binding [*out* *err*]
      (println "mutation-runner-adapter: feature" slug "ir" ir-path "test" test-name))
    (try
      (doseq [line (line-seq (java.io.BufferedReader. *in*))]
        (when-not (str/blank? line)
          (let [job (json/parse-string line)
                response (run-job ir-path test-name job)]
            (println (json/generate-string response))
            (flush))))
      (finally
        (spit ir-path original-ir)
        (binding [*out* *err*]
          (println "mutation-runner-adapter: restored" ir-path))))))

(apply -main *command-line-args*)
