# Contributing to rarch

First of all, thank you for considering contributing to **rarch**! It is people like you who make the open-source community such an amazing place.

## Code of Conduct
By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites
- **Rust Toolchain**: You will need the latest stable version of Rust. Install via [rustup](https://rustup.rs/).
- **Git**: Ensure Git is installed and configured on your machine.

### Development Workflow
1. **Fork the Repository**: Create a personal fork on GitHub.
2. **Clone the Fork**:
   ```bash
   git clone https://github.com/Serein-213/rarch.git
   cd rarch
   ```
3. **Create a Branch**: Use a descriptive name (e.g., `feat/simd-hashing` or `fix/journal-overflow`).
4. **Make Changes**: Ensure your code follows the existing style and conventions.
5. **Run Tests**:
   ```bash
   cargo test
   ```
6. **Linting**: We use `clippy` and `rustfmt` to maintain code quality.
   ```bash
   cargo fmt --all -- --check
   cargo clippy -- -D warnings
   ```

## Pull Request Process
- Ensure that each Pull Request addresses a single issue or feature.
- Update the documentation if your changes modify the user interface or configuration schema.
- All PRs require at least one maintainer's approval and must pass all CI checks before merging.

## Reporting Issues
Please use the provided [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) or [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) templates when opening issues.
