# Empirical Benchmark Specification for Proven: Multi-Regime Mutation Testing and AST Context Extraction Engine

Evaluating the operational performance of automated unit test generation engines for coding agents requires empirical benchmarks that move beyond coarse execution metrics. While existing suites such as SWE-bench and Defects4J measure high-level issue resolution and fault localization across large codebases, they lack granular instrumentation for Abstract Syntax Tree (AST) mutant kill rates, multi-hop call-graph propagation efficiency, and test generation bootstrapping from un-tested codebases.

**Proven** is a AST Context Extraction, Call-Graph Personalized PageRank (PPR), and Mutation Testing Verification Engine designed specifically for LLM coding agents. To empirically evaluate Proven across diverse code structures, execution speeds, context scales, and transfer learning capabilities, this foundational Phase-1 specification establishes 42 real-world open-source repositories partitioned across four core regimes, subsequently expanded in **v1.0.0 (`Polyglot-120`)** to **120 open-source repositories (`831` non-equivalent AST mutants)** across **6 Regimes** and **all 9 supported languages with `N ≥ 10` independent repositories per language** (`Python=40`, `Rust=10`, `Go=10`, `JavaScript=10`, `TypeScript=10`, `C=10`, `C++=10`, `Java=10`, `C#=10`), plus an **84-repository `2×2` cross-model factorial matrix (`681` AST mutants)**.

> [!IMPORTANT]
> **v1.0.0 Polyglot-120 & 84-Repository `2×2` Cross-Model Expansion**: For the complete 120-repository manifest (including Regime 4-Ext Cross-Language CLS Clusters, Regime 5 Hard-Tier Multi-Branch Polyglot, and Regime 6 Polyglot Systems & Enterprise Monoliths) and exact McNemar / Paired Sign Test results, see [`docs/EMPIRICAL_BENCHMARK_RESULTS.md`](EMPIRICAL_BENCHMARK_RESULTS.md), [`docs/ab_blind_agent_120_max10.json`](ab_blind_agent_120_max10.json), and [`docs/ab_2x2_matrix_84_max10.json`](ab_2x2_matrix_84_max10.json).

All selected repositories satisfy three strict technical requirements:
1. **Zero Infrastructure Dependency**: Core logic is testable via standard test runners (`pytest`, `cargo test`, `go test`, `node`) without requiring live Docker containers, external databases, cloud credentials, or GPU hardware.
2. **Language Balance**: Primary composition maintains a 70-80% Pure Python distribution to ensure fast `pytest` mutation execution cycles (< 1.5 seconds per target file), supplemented by Rust, Go, and TypeScript/JavaScript (20-30%) without heavy C/C++/Cython compilation overhead during setup.
3. **Rich AST Mutation Surface**: All target functions contain dense arithmetic operations, conditional logic, boolean state flags, explicit null guards, and state attribute mutations, providing a rich surface for mutant generation and verification.

---

## Regime 1: Small and Clean Repositories

### Operational Analysis
Regime 1 measures real AST mutant kill rates across small, well-structured projects. The primary objective is to evaluate the delta between baseline single-shot smoke test generation and Proven's mutation-verified Titanium loop. This regime specifically targets fine-grained boundary conditions, weak assertions, and immediate AST node mutation survival rates in localized code contexts. The repositories selected in this regime possess minimalist dependencies, deterministic build behavior, and total execution latencies under one second per module.

### Repository Manifest

| Repository Name | Language | Scale (Files / LOC) | Lightweight Setup Command | Candidate Target Functions / Methods |
| :--- | :--- | :--- | :--- | :--- |
| [pallets/werkzeug](https://github.com/pallets/werkzeug) | Python | 87 files / ~26,500 LOC | `pip install -e .` | `src/werkzeug/urls.py::uri_to_iri`<br>`src/werkzeug/datastructures/structures.py::Headers.add`<br>`src/werkzeug/http.py::parse_cookie` |
| [marshmallow-code/marshmallow](https://github.com/marshmallow-code/marshmallow) | Python | 42 files / ~15,200 LOC | `pip install -e .` | `src/marshmallow/validate.py::Range.__call__`<br>`src/marshmallow/fields.py::Field.serialize`<br>`src/marshmallow/schema.py::Schema._do_load` |
| [invl/retry](https://github.com/invl/retry) | Python | 6 files / ~450 LOC | `pip install -e .` | `retry/api.py::__retry_internal`<br>`retry/api.py::retry_call` |
| [jaraco/jaraco.text](https://github.com/jaraco/jaraco.text) | Python | 6 files / ~950 LOC | `pip install -e .` | `jaraco/text/__init__.py::WordSet.trim_right`<br>`jaraco/text/__init__.py::FoldedCase.index`<br>`jaraco/text/__init__.py::is_decodable` |
| [pypa/packaging](https://github.com/pypa/packaging) | Python | 40 files / ~14,500 LOC | `pip install -e .` | `src/packaging/specifiers.py::Specifier.contains`<br>`src/packaging/version.py::Version.__init__`<br>`src/packaging/markers.py::Marker.evaluate` |
| [jpadilla/pyjwt](https://github.com/jpadilla/pyjwt) | Python | 33 files / ~6,800 LOC | `pip install -e .` | `jwt/algorithms.py::HMACAlgorithm.prepare_key`<br>`jwt/api_jwt.py::PyJWT.decode`<br>`jwt/algorithms.py::HMACAlgorithm.verify` |
| [dateutil/dateutil](https://github.com/dateutil/dateutil) | Python | 62 files / ~24,000 LOC | `pip install -e .` | `src/dateutil/parser/_parser.py::_timelex.get_token`<br>`src/dateutil/rrule.py::rrule._iter` |
| [BurntSushi/tabwriter](https://github.com/BurntSushi/tabwriter) | Rust | 2 files / ~980 LOC | `cargo test` | `src/lib.rs::cell_widths`<br>`src/lib.rs::flush`<br>`src/lib.rs::add_bytes` |
| [sindresorhus/is-ip](https://github.com/sindresorhus/is-ip) | JavaScript | 3 files / ~220 LOC | `npm install` | `index.js::isIP`<br>`index.js::isIPv4` |
| [whitequark/ipaddr.js](https://github.com/whitequark/ipaddr.js) | JavaScript | 4 files / ~1,400 LOC | `npm install` | `lib/ipaddr.js::matchCIDR`<br>`lib/ipaddr.js::expandIPv6`<br>`lib/ipaddr.js::prefixLengthFromSubnetMask` |

```json
[
  {
    "repo_name": "https://github.com/pallets/werkzeug",
    "language": "Python",
    "scale": "87 files / ~26,500 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/werkzeug/urls.py::uri_to_iri",
      "src/werkzeug/datastructures/structures.py::Headers.add",
      "src/werkzeug/http.py::parse_cookie"
    ]
  },
  {
    "repo_name": "https://github.com/marshmallow-code/marshmallow",
    "language": "Python",
    "scale": "42 files / ~15,200 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/marshmallow/validate.py::Range.__call__",
      "src/marshmallow/fields.py::Field.serialize",
      "src/marshmallow/schema.py::Schema._do_load"
    ]
  },
  {
    "repo_name": "https://github.com/invl/retry",
    "language": "Python",
    "scale": "6 files / ~450 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "retry/api.py::__retry_internal",
      "retry/api.py::retry_call"
    ]
  },
  {
    "repo_name": "https://github.com/jaraco/jaraco.text",
    "language": "Python",
    "scale": "6 files / ~950 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "jaraco/text/__init__.py::WordSet.trim_right",
      "jaraco/text/__init__.py::FoldedCase.index",
      "jaraco/text/__init__.py::is_decodable"
    ]
  },
  {
    "repo_name": "https://github.com/pypa/packaging",
    "language": "Python",
    "scale": "40 files / ~14,500 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/packaging/specifiers.py::Specifier.contains",
      "src/packaging/version.py::Version.__init__",
      "src/packaging/markers.py::Marker.evaluate"
    ]
  },
  {
    "repo_name": "https://github.com/jpadilla/pyjwt",
    "language": "Python",
    "scale": "33 files / ~6,800 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "jwt/algorithms.py::HMACAlgorithm.prepare_key",
      "jwt/api_jwt.py::PyJWT.decode",
      "jwt/algorithms.py::HMACAlgorithm.verify"
    ]
  },
  {
    "repo_name": "https://github.com/dateutil/dateutil",
    "language": "Python",
    "scale": "62 files / ~24,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/dateutil/parser/_parser.py::_timelex.get_token",
      "src/dateutil/rrule.py::rrule._iter"
    ]
  },
  {
    "repo_name": "https://github.com/BurntSushi/tabwriter",
    "language": "Rust",
    "scale": "2 files / ~980 LOC",
    "install_cmd": "cargo test",
    "candidate_targets": [
      "src/lib.rs::cell_widths",
      "src/lib.rs::flush",
      "src/lib.rs::add_bytes"
    ]
  },
  {
    "repo_name": "https://github.com/sindresorhus/is-ip",
    "language": "JavaScript",
    "scale": "3 files / ~220 LOC",
    "install_cmd": "npm install",
    "candidate_targets": [
      "index.js::isIP",
      "index.js::isIPv4"
    ]
  },
  {
    "repo_name": "https://github.com/whitequark/ipaddr.js",
    "language": "JavaScript",
    "scale": "4 files / ~1,400 LOC",
    "install_cmd": "npm install",
    "candidate_targets": [
      "lib/ipaddr.js::matchCIDR",
      "lib/ipaddr.js::expandIPv6",
      "lib/ipaddr.js::prefixLengthFromSubnetMask"
    ]
  }
]
```

---

## Regime 2: Large and Monorepo Scale Repositories

### Operational Analysis
Regime 2 evaluates performance scalability across large codebases. It measures Proven's Scoped Fast-Path AST Extraction Latency ($\tau < 20\text{ms}$) against full repository graph parsing. It also tests context graph pruning using multi-hop Personalized PageRank (PPR), targeting a Context Compression Ratio ($\text{CCR} > 95\%$) while retaining relevant type, module, and function dependency paths. Repositories in this regime feature extensive AST node counts and deep inter-module dependencies, yet allow fast unit test execution on isolated target files.

### Repository Manifest

| Repository Name | Language | Scale (Files / LOC) | Lightweight Setup Command | Candidate Target Functions / Methods |
| :--- | :--- | :--- | :--- | :--- |
| [psf/black](https://github.com/psf/black) | Python | 86 files / ~34,000 LOC | `pip install -e .` | `src/black/linegen.py::LineGenerator.line`<br>`src/black/nodes.py::is_multiline_string`<br>`src/black/comments.py::generate_comments` |
| [pydantic/pydantic](https://github.com/pydantic/pydantic) | Python | 305 files / ~98,000 LOC | `pip install -e .` | `pydantic/main.py::BaseModel.__init__`<br>`pydantic/type_adapter.py::TypeAdapter.validate_python`<br>`pydantic/v1/validators.py::number_multiple_validator` |
| [pallets/flask](https://github.com/pallets/flask) | Python | 78 files / ~15,500 LOC | `pip install -e .` | `src/flask/app.py::Flask.dispatch_request`<br>`src/flask/ctx.py::AppContext.push`<br>`src/flask/app.py::Flask.make_shell_context` |
| [tiangolo/fastapi](https://github.com/tiangolo/fastapi) | Python | 1,180 files / ~96,000 LOC | `pip install -e .` | `fastapi/routing.py::APIRoute.get_route_handler`<br>`fastapi/dependencies/utils.py::solve_dependencies`<br>`fastapi/encoders.py::jsonable_encoder` |
| [pydicom/pydicom](https://github.com/pydicom/pydicom) | Python | 265 files / ~78,000 LOC | `pip install -e .` | `src/pydicom/fileset.py::FileSet.load`<br>`src/pydicom/dataset.py::Dataset.get`<br>`src/pydicom/dataelem.py::DataElement.value` |
| [sphinx-doc/sphinx](https://github.com/sphinx-doc/sphinx) | Python | 740 files / ~145,000 LOC | `pip install -e .` | `sphinx/environment/__init__.py::BuildEnvironment.resolve_references`<br>`sphinx/util/docutils.py::is_directive_registered` |
| [scrapy/scrapy](https://github.com/scrapy/scrapy) | Python | 365 files / ~65,000 LOC | `pip install -e .` | `scrapy/crawler.py::Crawler.crawl`<br>`scrapy/spidermiddlewares/depth.py::DepthMiddleware._init_depth` |
| [ipython/ipython](https://github.com/ipython/ipython) | Python | 320 files / ~88,000 LOC | `pip install -e .` | `IPython/core/interactiveshell.py::SeparateUnicode.validate`<br>`IPython/core/compilerop.py::CachingCompiler.ast_parse` |
| [astral-sh/ruff](https://github.com/astral-sh/ruff) | Rust | 2,950 files / ~420,000 LOC | `cargo test` | `crates/ruff_linter/src/rules/pyflakes/fixes.rs::remove_unused_positional_arguments_from_format_call`<br>`crates/ruff_python_ast/src/comparable.rs::from_float` |
| [gin-gonic/gin](https://github.com/gin-gonic/gin) | Go | 90 files / ~28,000 LOC | `go test ./...` | `tree.go::addRoute`<br>`context.go::Next`<br>`routergroup.go::handle` |

```json
[
  {
    "repo_name": "https://github.com/psf/black",
    "language": "Python",
    "scale": "86 files / ~34,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/black/linegen.py::LineGenerator.line",
      "src/black/nodes.py::is_multiline_string",
      "src/black/comments.py::generate_comments"
    ]
  },
  {
    "repo_name": "https://github.com/pydantic/pydantic",
    "language": "Python",
    "scale": "305 files / ~98,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "pydantic/main.py::BaseModel.__init__",
      "pydantic/type_adapter.py::TypeAdapter.validate_python",
      "pydantic/v1/validators.py::number_multiple_validator"
    ]
  },
  {
    "repo_name": "https://github.com/pallets/flask",
    "language": "Python",
    "scale": "78 files / ~15,500 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/flask/app.py::Flask.dispatch_request",
      "src/flask/ctx.py::AppContext.push",
      "src/flask/app.py::Flask.make_shell_context"
    ]
  },
  {
    "repo_name": "https://github.com/tiangolo/fastapi",
    "language": "Python",
    "scale": "1,180 files / ~96,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "fastapi/routing.py::APIRoute.get_route_handler",
      "fastapi/dependencies/utils.py::solve_dependencies",
      "fastapi/encoders.py::jsonable_encoder"
    ]
  },
  {
    "repo_name": "https://github.com/pydicom/pydicom",
    "language": "Python",
    "scale": "265 files / ~78,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/pydicom/fileset.py::FileSet.load",
      "src/pydicom/dataset.py::Dataset.get",
      "src/pydicom/dataelem.py::DataElement.value"
    ]
  },
  {
    "repo_name": "https://github.com/sphinx-doc/sphinx",
    "language": "Python",
    "scale": "740 files / ~145,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "sphinx/environment/__init__.py::BuildEnvironment.resolve_references",
      "sphinx/util/docutils.py::is_directive_registered"
    ]
  },
  {
    "repo_name": "https://github.com/scrapy/scrapy",
    "language": "Python",
    "scale": "365 files / ~65,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "scrapy/crawler.py::Crawler.crawl",
      "scrapy/spidermiddlewares/depth.py::DepthMiddleware._init_depth"
    ]
  },
  {
    "repo_name": "https://github.com/ipython/ipython",
    "language": "Python",
    "scale": "320 files / ~88,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "IPython/core/interactiveshell.py::SeparateUnicode.validate",
      "IPython/core/compilerop.py::CachingCompiler.ast_parse"
    ]
  },
  {
    "repo_name": "https://github.com/astral-sh/ruff",
    "language": "Rust",
    "scale": "2,950 files / ~420,000 LOC",
    "install_cmd": "cargo test",
    "candidate_targets": [
      "crates/ruff_linter/src/rules/pyflakes/fixes.rs::remove_unused_positional_arguments_from_format_call",
      "crates/ruff_python_ast/src/comparable.rs::from_float"
    ]
  },
  {
    "repo_name": "https://github.com/gin-gonic/gin",
    "language": "Go",
    "scale": "90 files / ~28,000 LOC",
    "install_cmd": "go test ./...",
    "candidate_targets": [
      "tree.go::addRoute",
      "context.go::Next",
      "routergroup.go::handle"
    ]
  }
]
```

---

## Regime 3: Dirty, Zero-Test, and Legacy Repositories

### Operational Analysis
Regime 3 evaluates noise filtering and test bootstrapping performance across standalone CLI scripts, legacy utilities, and minimal-test codebases. It measures Proven's ability to ignore untracked adversarial noise (such as `.venv`, `node_modules`, `target/debug`, broken syntax files, and `>1.5 MB` oversized generated dumps) while synthesizing unit tests from scratch. The goal is to lift stateful, side-effect-heavy utility functions from 0% unit coverage to 100% mutation-killed (Titanium grade) without existing harness assistance.

### Repository Manifest

| Repository Name | Language | Scale (Files / LOC) | Lightweight Setup Command | Candidate Target Functions / Methods |
| :--- | :--- | :--- | :--- | :--- |
| [kkroening/ffmpeg-python](https://github.com/kkroening/ffmpeg-python) | Python | 26 files / ~4,800 LOC | `pip install -e .` | `ffmpeg/_run.py::_get_output_args`<br>`ffmpeg/_ffmpeg.py::output` |
| [gleitz/howdoi](https://github.com/gleitz/howdoi) | Python | 19 files / ~2,400 LOC | `pip install -e .` | `howdoi/howdoi.py::_get_answers`<br>`howdoi/howdoi.py::get_text` |
| [docopt/docopt](https://github.com/docopt/docopt) | Python | 16 files / ~1,800 LOC | `pip install -e .` | `docopt.py::LeafPattern.match`<br>`docopt.py::Pattern.fix_repeating_arguments` |
| [keleshev/schema](https://github.com/keleshev/schema) | Python | 3 files / ~1,100 LOC | `pip install -e .` | `schema/__init__.py::Or.validate`<br>`schema/__init__.py::Schema.validate` |
| [gruns/icecream](https://github.com/gruns/icecream) | Python | 9 files / ~1,200 LOC | `pip install -e .` | `icecream/icecream.py::safe_pformat`<br>`icecream/icecream.py::has_non_ascii_chars` |
| [termcolor/termcolor](https://github.com/termcolor/termcolor) | Python | 5 files / ~520 LOC | `pip install -e .` | `src/termcolor/termcolor.py::can_colorize`<br>`src/termcolor/termcolor.py::colored` |
| [awss1i/assay](https://github.com/awss1i/assay) | Python | 34 files / ~4,200 LOC | `pip install -e .` | `src/assay/qa.py::Result.render`<br>`src/assay/links.py::attach` |
| [ellielle/tfdid](https://github.com/ellielle/tfdid) | Python | 7 files / ~650 LOC | `pip install -e .` | `classes/parser.py::DirectoryScan._convert_block_size`<br>`classes/parser.py::DirectoryScan.run_dir_scan` |
| [lukeed/ms](https://github.com/lukeed/ms) | JavaScript | 4 files / ~250 LOC | `npm install` | `src/index.js::fmt`<br>`src/index.js::parse` |
| [dtolnay/itoa](https://github.com/dtolnay/itoa) | Rust | 4 files / ~450 LOC | `cargo test` | `src/lib.rs::write`<br>`src/lib.rs::format` |

```json
[
  {
    "repo_name": "https://github.com/kkroening/ffmpeg-python",
    "language": "Python",
    "scale": "26 files / ~4,800 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "ffmpeg/_run.py::_get_output_args",
      "ffmpeg/_ffmpeg.py::output"
    ]
  },
  {
    "repo_name": "https://github.com/gleitz/howdoi",
    "language": "Python",
    "scale": "19 files / ~2,400 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "howdoi/howdoi.py::_get_answers",
      "howdoi/howdoi.py::get_text"
    ]
  },
  {
    "repo_name": "https://github.com/docopt/docopt",
    "language": "Python",
    "scale": "16 files / ~1,800 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "docopt.py::LeafPattern.match",
      "docopt.py::Pattern.fix_repeating_arguments"
    ]
  },
  {
    "repo_name": "https://github.com/keleshev/schema",
    "language": "Python",
    "scale": "3 files / ~1,100 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "schema/__init__.py::Or.validate",
      "schema/__init__.py::Schema.validate"
    ]
  },
  {
    "repo_name": "https://github.com/gruns/icecream",
    "language": "Python",
    "scale": "9 files / ~1,200 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "icecream/icecream.py::safe_pformat",
      "icecream/icecream.py::has_non_ascii_chars"
    ]
  },
  {
    "repo_name": "https://github.com/termcolor/termcolor",
    "language": "Python",
    "scale": "5 files / ~520 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/termcolor/termcolor.py::can_colorize",
      "src/termcolor/termcolor.py::colored"
    ]
  },
  {
    "repo_name": "https://github.com/awss1i/assay",
    "language": "Python",
    "scale": "34 files / ~4,200 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/assay/qa.py::Result.render",
      "src/assay/links.py::attach"
    ]
  },
  {
    "repo_name": "https://github.com/ellielle/tfdid",
    "language": "Python",
    "scale": "7 files / ~650 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "classes/parser.py::DirectoryScan._convert_block_size",
      "classes/parser.py::DirectoryScan.run_dir_scan"
    ]
  },
  {
    "repo_name": "https://github.com/lukeed/ms",
    "language": "JavaScript",
    "scale": "4 files / ~250 LOC",
    "install_cmd": "npm install",
    "candidate_targets": [
      "src/index.js::fmt",
      "src/index.js::parse"
    ]
  },
  {
    "repo_name": "https://github.com/dtolnay/itoa",
    "language": "Rust",
    "scale": "4 files / ~450 LOC",
    "install_cmd": "cargo test",
    "candidate_targets": [
      "src/lib.rs::write",
      "src/lib.rs::format"
    ]
  }
]
```

---

## Regime 4: Cross-Repository Generalization Clusters

### Operational Analysis
Regime 4 measures zero-shot AST Archetype recall and cross-domain convergence speedup. It evaluates how testing rules consolidated in Domain A transfer to unseen codebases in Domain B1 and Domain B2 when domain-specific vocabularies share no overlap.

Proven extracts meta-axioms across four generalized AST structural archetypes:
1. `NULL_GUARD`: Reference validation, empty checks, and optional attribute resolution.
2. `DIV_ZERO_OR_BOUNDARY`: Precision thresholds, numeric limits, and array index range guards.
3. `STATE_SIDE_EFFECT`: In-place collections, property assignments, and state transitions.
4. `EXCEPTION_CONTRACT`: Explicit error handling, custom exception hierarchies, and fallback branches.

### Domain Clusters
- **Cluster A**: FinTech, Numeric Precision, and Rate Limiting. Focuses on floating-point boundary arithmetic, decimal calculations, and dynamic quota allocation logic.
- **Cluster B**: Healthcare, Scientific Data, and Telemetry Parsing. Focuses on structured array transformations, binary format decoders, and data integrity checks.
- **Cluster C**: Security, Identity, Policy, and Cryptographic Operations. Focuses on permission trees, access checks, and token decoding pipelines.
- **Cluster D**: Scheduling, Workflow Queues, and State Machines. Focuses on state transitions, cron expressions, and execution loops.

### Repository Manifest

| Domain Cluster | Repository Name | Language | Scale | Lightweight Setup Command | Candidate Target Functions / Methods |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Cluster A** | [mpmath/mpmath](https://github.com/mpmath/mpmath) | Python | 115 files / ~48,000 LOC | `pip install -e .` | `mpmath/libmp/libmpf.py::mpf_mul`<br>`mpmath/ctx_mp_python.py::_mpf.mpf_convert_arg` |
| **Cluster A** | [ulule/limiter](https://github.com/ulule/limiter) | Go | 28 files / ~2,400 LOC | `go test ./...` | `drivers/middleware/fasthttp/middleware.go::Handle`<br>`drivers/store/memory/cache.go::startCleaner` |
| **Cluster A** | [animir/node-rate-limiter-flexible](https://github.com/animir/node-rate-limiter-flexible) | JavaScript | 68 files / ~8,200 LOC | `npm install` | `lib/RateLimiterMemory.js::consume`<br>`lib/RateLimiterDrizzle.js::_getRateLimiterRes` |
| **Cluster B** | [pydicom/pydicom](https://github.com/pydicom/pydicom) | Python | 265 files / ~78,000 LOC | `pip install -e .` | `src/pydicom/valuerep.py::validate_value`<br>`src/pydicom/multival.py::MultiValue.append` |
| **Cluster B** | [biopython/biopython](https://github.com/biopython/biopython) | Python | 540 files / ~195,000 LOC | `pip install -e .` | `Bio/Align/__init__.py::MultipleSeqAlignment.__init__`<br>`Bio/Seq.py::Seq.translate` |
| **Cluster B** | [influxdata/line-protocol](https://github.com/influxdata/line-protocol) | Go | 16 files / ~3,200 LOC | `go test ./...` | `lineprotocol/decoder.go::Next`<br>`lineprotocol/byteset.go::newByteSetRange` |
| **Cluster C** | [casbin/pycasbin](https://github.com/casbin/pycasbin) | Python | 68 files / ~8,500 LOC | `pip install -e .` | `casbin/model/model.py::Model.add_def`<br>`casbin/enforcer.py::Enforcer.has_role_for_user` |
| **Cluster C** | [paramiko/paramiko](https://github.com/paramiko/paramiko) | Python | 76 files / ~26,000 LOC | `pip install -e .` | `paramiko/auth_handler.py::AuthHandler.wait_for_response`<br>`paramiko/kex_curve25519.py::KexCurve25519.parse_next` |
| **Cluster C** | [open-policy-agent/opa](https://github.com/open-policy-agent/opa) | Go | 920 files / ~240,000 LOC | `go test ./...` | `ast/ast.go::MustCompileModulesWithOpts`<br>`v1/ast/compile.go::WithStageAfter` |
| **Cluster D** | [pytransitions/transitions](https://github.com/pytransitions/transitions) | Python | 28 files / ~9,800 LOC | `pip install -e .` | `transitions/core.py::Machine.add_transition`<br>`transitions/core.py::Transition.execute` |
| **Cluster D** | [celery/kombu](https://github.com/celery/kombu) | Python | 112 files / ~28,000 LOC | `pip install -e .` | `kombu/utils/scheduling.py::fair_cycle.get`<br>`kombu/transport/virtual/base.py::Channel._put` |
| **Cluster D** | [gorhill/cronexpr](https://github.com/gorhill/cronexpr) | Go | 9 files / ~1,900 LOC | `go test ./...` | `cronexpr_next.go::calculateActualDaysOfMonth`<br>`cronexpr.go::Next` |

```json
[
  {
    "repo_name": "https://github.com/mpmath/mpmath",
    "language": "Python",
    "scale": "115 files / ~48,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "mpmath/libmp/libmpf.py::mpf_mul",
      "mpmath/ctx_mp_python.py::_mpf.mpf_convert_arg"
    ]
  },
  {
    "repo_name": "https://github.com/ulule/limiter",
    "language": "Go",
    "scale": "28 files / ~2,400 LOC",
    "install_cmd": "go test ./...",
    "candidate_targets": [
      "drivers/middleware/fasthttp/middleware.go::Handle",
      "drivers/store/memory/cache.go::startCleaner"
    ]
  },
  {
    "repo_name": "https://github.com/animir/node-rate-limiter-flexible",
    "language": "JavaScript",
    "scale": "68 files / ~8,200 LOC",
    "install_cmd": "npm install",
    "candidate_targets": [
      "lib/RateLimiterMemory.js::consume",
      "lib/RateLimiterDrizzle.js::_getRateLimiterRes"
    ]
  },
  {
    "repo_name": "https://github.com/pydicom/pydicom",
    "language": "Python",
    "scale": "265 files / ~78,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "src/pydicom/valuerep.py::validate_value",
      "src/pydicom/multival.py::MultiValue.append"
    ]
  },
  {
    "repo_name": "https://github.com/biopython/biopython",
    "language": "Python",
    "scale": "540 files / ~195,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "Bio/Align/__init__.py::MultipleSeqAlignment.__init__",
      "Bio/Seq.py::Seq.translate"
    ]
  },
  {
    "repo_name": "https://github.com/influxdata/line-protocol",
    "language": "Go",
    "scale": "16 files / ~3,200 LOC",
    "install_cmd": "go test ./...",
    "candidate_targets": [
      "lineprotocol/decoder.go::Next",
      "lineprotocol/byteset.go::newByteSetRange"
    ]
  },
  {
    "repo_name": "https://github.com/casbin/pycasbin",
    "language": "Python",
    "scale": "68 files / ~8,500 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "casbin/model/model.py::Model.add_def",
      "casbin/enforcer.py::Enforcer.has_role_for_user"
    ]
  },
  {
    "repo_name": "https://github.com/paramiko/paramiko",
    "language": "Python",
    "scale": "76 files / ~26,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "paramiko/auth_handler.py::AuthHandler.wait_for_response",
      "paramiko/kex_curve25519.py::KexCurve25519.parse_next"
    ]
  },
  {
    "repo_name": "https://github.com/open-policy-agent/opa",
    "language": "Go",
    "scale": "920 files / ~240,000 LOC",
    "install_cmd": "go test ./... ",
    "candidate_targets": [
      "ast/ast.go::MustCompileModulesWithOpts",
      "v1/ast/compile.go::WithStageAfter"
    ]
  },
  {
    "repo_name": "https://github.com/pytransitions/transitions",
    "language": "Python",
    "scale": "28 files / ~9,800 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "transitions/core.py::Machine.add_transition",
      "transitions/core.py::Transition.execute"
    ]
  },
  {
    "repo_name": "https://github.com/celery/kombu",
    "language": "Python",
    "scale": "112 files / ~28,000 LOC",
    "install_cmd": "pip install -e .",
    "candidate_targets": [
      "kombu/utils/scheduling.py::fair_cycle.get",
      "kombu/transport/virtual/base.py::Channel._put"
    ]
  },
  {
    "repo_name": "https://github.com/gorhill/cronexpr",
    "language": "Go",
    "scale": "9 files / ~1,900 LOC",
    "install_cmd": "go test ./...",
    "candidate_targets": [
      "cronexpr_next.go::calculateActualDaysOfMonth",
      "cronexpr.go::Next"
    ]
  }
]
```

---

## Evaluation Metrics and Execution Protocols

The benchmark measures Proven across three primary operational dimensions:

### Scoped AST Extraction and Context Compression Efficiency
Context generation performance is measured using extraction latency ($\tau$) and Context Compression Ratio ($\text{CCR}$):

$$\text{CCR} = \left( 1 - \frac{\text{Tokens in Scoped Context Graph}}{\text{Tokens in Full Repository Abstract Syntax Tree}} \right) \times 100\%$$

Proven must maintain $\tau < 20\text{ms}$ on Regime 2 monorepos while achieving $\text{CCR} > 95\%$ relative to flat repository context retrieval.

### Mutation Verification and Mutant Survival Rates
For a target function $f$, Proven generates $M$ syntax mutations ($m_1, m_2, \dots, m_M$) modifying relational operators, arithmetic signs, and boolean guards. The Mutant Kill Rate ($K_R$) is calculated as:

$$K_R = \frac{\sum_{i=1}^{M} \mathbb{I}(\text{Test Suite Fails on } m_i)}{M} \times 100\%$$

In Regime 1 and Regime 3, baseline smoke test suites generated by single-shot LLMs are compared against Proven's mutation-verified suites.

### Bootstrapping Coverage Delta
In Regime 3 (zero-test and legacy codebases), performance is evaluated based on noise immunity ($0$ parser crashes, $100\%$ noise directory and oversized file exclusion) and Titanium status convergence speed when bootstrapping tests from $0$ existing unit tests.

---

## Summary of Benchmark Suite Characteristics

1. **Structured Evaluation Scope**: The 42 curated repositories provide an isolated, deterministic environment for evaluating AST context extraction, personalized PageRank graph compression, and mutation testing verification.
2. **Execution Efficiency**: Emphasizing lightweight setups and isolated test runners ensures fast evaluation cycles without external service dependencies or infrastructure overhead.
3. **Cross-Domain Adaptability**: Selecting targets across distinct architectural domains enables evaluation of zero-shot transfer learning for AST mutation testing rules across disparate codebases.
