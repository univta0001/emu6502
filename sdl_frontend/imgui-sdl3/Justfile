default:
    just --list

lint:
    cargo fmt --check
    cargo check --all-targets --examples --tests --all-features
    cargo clippy --all-targets --examples --tests --all-features -- --deny warnings
    typos
    cargo deny --all-features check
    taplo check *.toml
    find shaders/ -iname *.vert -o -iname *.frag -o -iname *.comp -o -iname *.glsl | xargs clang-format --dry-run

fmt:
    cargo fmt
    cargo fix --all-targets --examples --tests --allow-dirty --all-features
    cargo clippy --all-targets --fix --examples --tests --allow-dirty --all-features -- --deny warnings
    typos --write-changes
    cargo deny --log-level off check --show-stats
    taplo format *.toml
    find shaders/ -iname *.vert -o -iname *.frag -o -iname *.comp -o -iname *.glsl | xargs clang-format -i

build:
    cargo build --examples --all-targets --all-features