quickcheck:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    just veryquickcheck
    just clippy
    just test
    just nostd
    just doc-tests

veryquickcheck:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    just codegen-check
    just rustfmt
    just absolve

nostd:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;33m🧪 Checking without std...\033[0m"
    cargo check --no-default-features -p facet-core
    cargo check --no-default-features -p facet
    cargo check --no-default-features -p facet-peek
    cargo check --no-default-features -p facet-poke

ci:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    just quickcheck
    just miri
    echo -e "\033[1;34m📝 Running cargo fmt in check mode...\033[0m"
    cargo fmt --all -- --check

rustfmt:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;34m📝 Checking code formatting...\033[0m"
    cargo fmt --all -- --check

clippy:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;35m🔍 Running Clippy on all targets...\033[0m"
    cargo clippy --all-targets -- -D warnings

test *args:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;33m🏃 Running all but doc-tests with nextest...\033[0m"
    cargo nextest run {{args}} < /dev/null
    echo -e "\033[1;33m✅ Good good!\033[0m"

doc-tests:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;36m📚 Running documentation tests...\033[0m"
    cargo test --doc

codegen:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    cp templates/README.md.j2 facet/templates/README.md.j2
    cargo run -p facet-codegen

codegen-check:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    cargo run -p facet-codegen -- --check

rustfmt-fix:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    echo -e "\033[1;34m📝 Fixing code formatting...\033[0m"
    cargo fmt --all

miri *args:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    export CARGO_TARGET_DIR=target/miri
    export RUSTUP_TOOLCHAIN=nightly-2025-04-05
    echo -e "\033[1;31m🧪 Running tests under Miri...\033[0m"
    rustup toolchain install
    rustup component add miri rust-src
    cargo miri nextest run {{args}}

absolve:
    #!/usr/bin/env -S bash -euo pipefail
    source .envrc
    if ! cargo tree -i syn 2>/dev/null | grep -q .; then
    echo -e "\033[38;2;255;255;255;48;2;0;0;0m free of \033[38;2;255;255;255;48;2;255;105;180m syn \033[38;2;255;255;255;48;2;0;0;0m\033[0m"
    else
    echo -e "\033[1;31m❌ 'syn' found in dependency tree. Here's what's using 'syn':\033[0m"
    cargo tree -i syn -e features
    exit 1
    fi

ship:
    #!/usr/bin/env -S /bin/bash -eux
    release-plz update
    git add .
    git commit
    git push
    just release

release:
    # This works around a bug in release-plz: it does not publish dev dependencies in the right order.
    cargo publish -p facet-samplelibc || true
    release-plz release --backend github --git-token $(gh auth token)
