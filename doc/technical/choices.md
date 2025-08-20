# Technical choices

## Programming language

### Context

The project requires a programming language that balances performance, security and development velocity for a 2-week development timeline with a 4-person team.

### Rationale

Rust has been selected as the primary programming language due to its growing adoption as the industry standards for security-critical applications. While it doesn't eliminate all security vulnerabilities, Rust's ownership system and memory safety guarantees significantly reduce common attack vectors. It eliminates buffer overflows, use-after-free and null pointer dereferences at compile time. It is thread safe and prevents data races through the type system.

In addition to this 'Security-firts design' features, Rust's performances can be compared to C/C++ performances. Its strong type system and comprehensive error handling reduce runtime failures and improve code maintainability.
