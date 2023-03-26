# Implementing a distributed key-value store in Rust

This is a simple distributed key-value store implemented in Rust, created for learning purposes

## Checklist

- [x] Implement the database system

  - [x] Have a database interface, methods GET and PUT
  - [x] Make database accessible via TCP

- [x] Implement API endpoint that accepts requests and forwards them to the database

  - [x] TCP based API
  - [x] HTTP based API

- [ ] Implement watchdog for service discovery

  - [x] One watchdog instance, multiple instances connecting to it
  - [x] Broadcast instances list to each instance
  - [x] Perform heartbeat test from instances to watchdog, exit when watchdog is dead
  - [ ] Perform heartbeat test from watchdog to instances

- [ ] Implement Raft algorithm
  - [ ] Service discovery using a watchdog
  - [ ] Peer communication
  - [ ] Leader election
  - [ ] Log replication
  - [ ] Client interaction
