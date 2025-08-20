# `miel` honeypot software requirements

The following document outlines the functional and non-functional requirements
for the `miel` honeypot software. The goal of this software is to expose various
containerized services to attract and analyze malicious traffic.

## Functional requirements

The software must meet the following functional requirements:

- Identify the requested service.
- Allow the user to expose a service to honeypot that must be:
  - Configured through text files.
  - Containerizable.
  - Interactive, if applicable.
  - Exposable to the internet.
- Expose containerized service uniquely attached to a session and in isolation.
- Obfuscate the containerized service as a tangible part of an infrastructure.
- Record and store the metadata and payloads of the session.
- Record and store a log of the interactions with the service.
- Output stored data to a file or a database.
- Filter the use of specific IP ranges, ports, protocols and services.
- Present a web interface to view the honeypot status and browse the data.
- Configurable through text files or command line arguments.
- Deployable as a single binary.

## Non-functional requirements

The software must meet the following non-functional requirements:

- Run on a single host machine with at least 16GB of RAM and 4 CPU cores.
- Be able to deploy lightweight containers using Linux namespaces.
- The software must not introduce a network latency greater than 100ms when an
  attacker interacts with the honeypot.
- The software must scale the number of available service sessions for attackers
  based on the amount of requests for a service such that the average response
  time is less than 100ms.
- Output the data to a text file or store the data in persistent storage.
- Define and document a protocol for service configuration.
- Define and document default values for the configuration of the software.
- Document the codebase and architecture of the software.
- Document the API and the usage of the web interface.
- Implement automated testing to ensure the software meets the functional and
  non-functional requirements.
