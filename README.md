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
  - [Gift](#gift)
    - [Create Gift](#create-gift)

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

Remove signer:

```bash
cargo test remove_signer_success --release -- --nocapture --test-threads=1
```

Remove non signer (should fail):

```bash
cargo test remove_signer_with_non_signer --release -- --nocapture --test-threads=1
```

Remove signer causing threshold unreachable (should fail):

```bash
cargo test remove_signer_causing_threshold_unreachable --release -- --nocapture --test-threads=1
```

### Change Threshold

Change threshold:

```bash
cargo test change_threshold_success --release -- --nocapture --test-threads=1
```

Change threshold with same threshold (should fail):

```bash
cargo test change_threshold_with_same_threshold --release -- --nocapture --test-threads=1
```

Change threshold with invalid threshold (should fail):

```bash
cargo test change_threshold_with_invalid_threshold --release -- --nocapture --test-threads=1
```

---

## Gift

### Create Gift

Create and consume gift success:

```bash
cargo test create_and_open_gift_success --release -- --nocapture --test-threads=1
```

Create and consume gift fail with wrong secret:

```bash
cargo test open_gift_with_wrong_secret --release -- --nocapture --test-threads=1
```

---
