# How to configure sops to encrypt secrets

Configuration files and keys inside the `secrets` folder can be used as a plaintext or encrypted file via sops tool.

**It's up to the user to decide** how to handle secrets and keys on their own environments. For more information - please check [documentation](https://github.com/getsops/sops)

**NOTE:**
- `<blockchain>` is a placeholder for the custom blockchain the user wants to run against
- `<PGP_KEY>` is a placeholder for the user's PGP key

## Install and configure sops
1. Install sops - `brew install sops`
2. Generate new PGP key or use existing one
3. Upload your pgp public key to one of the pgp key servers, e.g. pgp.mit.edu.
4. Configure the remote key service. It can be AWS KMS, GCP KMS, Azure Vault, Hashicorp Vault or [[other](https://github.com/getsops/sops?tab=readme-ov-file#usage)]
5. Generate AWS KMS key - `aws kms create-key --tags TagKey=Purpose,TagValue=Test --description "Test key"`
6. Add your PGP and service keys to `.sops.yaml`. E.g., for AWS KMS it will look like:

```
creation_rules:
    ...
    - path_regex: ^secrets\/<blockchain>(\/.*)?$
      kms: >-
        arn:aws:kms:<YOUR_CONFIGURATION>
      pgp: >-
        <PGP_KEY>
```
7. Install `signageos.signageos-vscode-sops` VS Code extension. It will perform encrypt and decrypt operations

If, for whatever reason, automatic encryption doesn't work you can always perform it manually:
```bash
sops -e -i <path to the new secret file>
```

> Do not forget to run all tests with the `--decrypt` parameter to perform automatic decryption of keys in `secrets` folder.
