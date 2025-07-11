# MASM Weighted Multisig

A minimal example for compiling, deploying, and testing MASM contracts & notes, featuring a weighted multisig account.

---

## Table of Contents

- [MASM Weighted Multisig](#masm-weighted-multisig)
  - [Table of Contents](#table-of-contents)
  - [Running All Tests](#running-all-tests)
  - [Deployment](#deployment)
  - [Signer Management](#signer-management)
    - [Add Signer](#add-signer)
    - [Remove Signer](#remove-signer)
    - [Change Threshold](#change-threshold)
  - [Other Features](#other-features)

---

## Running All Tests

```bash
cargo test --release -- --nocapture --test-threads=1
```

---

## Deployment

Deploy the weighted multisig contract:

```bash
cargo test deploy_multisig --release -- --nocapture --test-threads=1
```

---

## Signer Management

### Add Signer

Add a new signer:

```bash
cargo test add_signer_success --release -- --nocapture --test-threads=1
```

Add a new signer with the same public key (should fail):

```bash
cargo test add_signer_with_same_public_key --release -- --nocapture --test-threads=1
```

Add a new signer with invalid weight (should fail):

```bash
cargo test add_signer_with_invalid_weight --release -- --nocapture --test-threads=1
```

### Remove Signer

<!-- Add instructions here when implemented -->

### Change Threshold

<!-- Add instructions here when implemented -->

---

## Other Features

<!-- Add more sections as your project grows -->
