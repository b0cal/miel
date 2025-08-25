# Technical choices

## Programming language

### Context

The project requires a programming language that balances performance, security
and development velocity for a 2-week development timeline with a 4-person team.

### Rationale

`Rust` has been selected as the primary programming language due to its growing
adoption as the industry standards for security-critical applications. While it
doesn't eliminate all security vulnerabilities, Rust's ownership system and
memory safety guarantees significantly reduce common attack vectors. It
eliminates buffer overflows, use-after-free and null pointer dereferences at
compile time. It is thread safe and prevents data races through the type system.

In addition to this 'Security-firts design' features, Rust's performances can be
compared to C/C++ performances. Its strong type system and comprehensive error
handling reduce runtime failures and improve code maintainability.

## Asynchronous runtime

### Context

Our software needs a way of handling concurrent network connections properly.

### Rationale

Handling natively this task can be tedious an time-consuming, that's why using
`tokio` as a asynchronous runtime can be life saving. It securely adresses
asynchrony in providing us an asynchronous version of the Rust's standard
library. Its speed depends mostly on the use of the async/await language feature
and lets us handle multiple simultaneous connections.

## Container service

### Context

To host the masquerade service we need an lightweight and efficient way of
pulling small containers exposing vulnerabilities on the appropriate service.

### Rationale

Using VMs would be too resource consuming, that's why we went for a container
solution. Docker could have been a good fit but we wanted to find something even
lighter than this an with a simpler API than Docker's.

`systemd-nspawn` is a lightweight container runtime ensuring isolation using
Linux namespaces and cgroups. It integrates well into systemd-based Linux
system, is manageable as a systemd service and guarantees a fast startup time.

## Codebase documentation

### Context

Maintaining a good codebase documentation leads to better collaboration on the
project and is easier for people to join the project on the way. Consistency is
key which means we need to use a standardized why of documenting our codebase.

### Rationale

Rust's package manager, cargo, embeds a documentation manager in its toolchain,
`rustdoc`. With simple commenting syntax that supports Markdown formatting, it
allows us to generate the whole codebase documentation at once and lets us
access an HTML riced version of it.

## Initially supported services

### Context

We need to ship the product with at least two compatible services in order to
have a POC.

### Rationale

`ssh` and `http` are the most scanned and prone to vulnerability services.
That's why these are the first two services we support on this application.

## Project Management

### Context

In order to track progress through the project, we need a project management
system allowing us to know what needs to be done and what has been done.
Moreover submitting new features should be done in a standardized why to keep
better track of the history.

### Rationale

As Github serves us as our codebase repository and in order not to multiply the
tools we are using, we went for the Github Project embedded solution. That lets
us manage the SCRUM with a Kanban, view it as a roadmap and so on. In addition,
to keep issues and PR submitting we've set some PR and issues template that can
be directly loaded when opening a new one.
