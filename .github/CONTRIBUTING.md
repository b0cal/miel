## Contributing to miel

Thank you for showing interest in this project ! We appreciate your time and effort in helping make this project better. This guide will help you understand our development process and how to contribute effectively.


## Getting Started

### Prerequisites

Before you begin, ensure you have:
- **Operating System**: Debian or Fedora (x86_64)
- **Container Support**: systemd-nspawn compatibility
- **Git**: For version control

### First Steps
1. **Fork the repository** on Github
2. **Clone your fork** locally:
```bash
git clone https://github.com/your-username/miel.git
cd miel
```
3. Add upstream remote:
```bash
git remote add upstream https://github.com/miel/miel.git
```

##  Development Environment

### Installation

1. **Navigate to the project directory**:
```bash
cd miel
```
2. **Run the setup scripts:**
```bash
# Install dependencies and configure environment
./scripts/setup_markdown.sh
./scripts/setup_node.sh
./scripts/setup_rust.sh
```

### Environment Configuration
- Ensure systemd-nspawn is properly configured
- Check that all dependencies are correctly installed
- Verify your development environment matches our requirements

## Contribution Workflow

### 1. Choose Your Contribution

- Browse [open issues](https://github.com/b0cal/miel/issues) for tasks to work on
- Check the [project roadmap](https://github.com/orgs/b0cal/projects/1/views/4) for upcoming features
- Propose new features by opening an issue first

### 2. Create a Branch

```bash
# Keep your fork updated
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git switch -c feature/your-feature-name
# or
git switch -c fix/bug-description
```

### 3. Develop Your Changes

- Follow our [coding guidelines](/docs/workflow/development.md)
- Write comprehensive tests for your changes
- Update documentation as needed
- Commit your changes with clear, descriptive messages

### 4. Test Your Changes

```bash
# Run the full test suite
cargo make test

# Check code quality
cargo code-quality
cargo make lint
cargo make fmt
cargo make fmt-fix
```

## Coding Guidelines

Please refer to our detailed [development guidelines](/docs/workflow/development.md) for:

- Code style and formatting standards
- Architecture patterns and best practices
- Naming conventions
- Performance considerations
- Security requirements

### Key Points

- **Consistency**: Follow existing code patterns and style
- **Clarity**: Write self-documenting code with clear variable names
- **Performance**: Consider performance implications of your changes
- **Security**: Follow security best practices
- **Documentation**: Comment complex logic and update relevant docs

## Testing Requirements

### Unit Tests

- All new features must include comprehensive unit tests
- Maintain or improve existing code coverage
- Tests should be fast, reliable, and independent

### Integration Tests

- Add integration tests for features that interact with external systems
- Test edge cases and error conditions

## Documentation Standards

### Code Documentation

- Use clear, descriptive comments for complex logic
- Document public APIs and interfaces
- Include usage examples where appropriate

### User Documentation

- Update user guides for new features
- Include configuration examples
- Update CLI help text and man pages

### API Documentation

- Document all public APIs
- Include request/response examples
- Specify error conditions and codes

## Pull Request Process

### Before Submitting

1. **Review our [PR Guidelines](/doc/workflow/development.md#code-review-and-pull-requests)**
2. **Ensure your branch is up to date**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```
3. **Run the full test suite**
4. **Update documentation** if needed
5. **Write a clear PR description**

### PR Template

When creating your PR, please:

- Use the provided PR template
- Describe what changes you made and why
- Reference related issues
- Include testing instructions
- Add screenshots for UI changes
- List any breaking changes

### Review Process

1. **Automated checks**: All CI checks must pass
2. **Code review**: At least one team member will review your PR
3. **Testing**: Reviewers will test your changes
4. **Feedback**: Address any requested changes promptly
5. **Approval**: Once approved, your PR will be merged

## Issue Reporting

### Bug Reports

When reporting bugs, please include:

- **Environment details**: OS, version, configuration
- **Steps to reproduce**: Clear, numbered steps
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Error messages**: Full error output
- **Logs**: Relevant log files

### Feature Requests

For new features:

- **Use case**: Describe why this feature is needed
- **Proposed solution**: Your suggested approach
- **Alternatives**: Other solutions you've considered
- **Impact**: How this affects existing functionality

## Release Process

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):
- **Major**: Breaking changes
- **Minor**: New features, backwards compatible
- **Patch**: Bug fixes, backwards compatible

## Getting Help

If you need help:

1. Check existing documentation and issues
4. Be patient and respectful when asking for help

## Recognition

Contributors are recognized in:
- Release notes for significant contributions
- The project's contributors list
- Special recognition for long-term contributors

---

> "Every time you find yourself here, it's because you chose to come back"
> 
> Mark S.

Thank you for contributing to miel! Your efforts help make this project better for everyone.
