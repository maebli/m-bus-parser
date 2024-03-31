# Contributing

This docuement is new, so its much appreciated, if you ask question or give feedback. Don't hesitate to open an issue.

This document aims to provide a guideline for people wanting to contribute to the project. It does not aim to describe the project, please see the README.md for this. 

# Conduct

Code of conduct is taken form the [rust project code of conduct](https://www.rust-lang.org/policies/code-of-conduct).


# Getting started

## Know the basics of rust

There are a wide variety of resources available to learn rust. The following resources may be helpful:

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

## Understanding the code

The code and its function may be confusing at first. The following resouces may help you understand the code:

- [m-bus website](https://m-bus.com/documentation). This document is a good starting point for understanding the protocol. It is outdated but still useful starting point. The m-bus norm is mostly backwards compatible. 


## Making changes to the code
1. (Install the rust toolchain)[https://www.rust-lang.org/tools/install]
2. Install your favorite IDE such as vscode, intellij or neovim
3. Install git and clone the repository `git clone git@github.com:maebli/m-bus-parser.git`
4. Run the tests `cargo test` and see if you can run the tests


# How to contribute

1. Fork the repository
2. Create a branch `git checkout -b feature/your-feature`
3. Make your changes
4. Run the tests `cargo test`
5. Commit your changes `git commit -m "feat: your feature"`
6. Push your changes `git push origin feature/your-feature`
7. Create a pull request


