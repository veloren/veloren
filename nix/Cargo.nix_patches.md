Apply these instructions after generating the `Cargo.nix` file.

- Find `veloren-common` crate in `Cargo.nix`:
	- Comment the `optional = true;` line for "structopt" and "csv" dependencies.
	- See [this issue](https://github.com/kolloch/crate2nix/issues/129) on `crate2nix` repository for more info.
	- Note that the suggested workaround in the issue **does not** work for us.
