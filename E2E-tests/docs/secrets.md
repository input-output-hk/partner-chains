# Secrets
In order to run tests, one has to be able to decrypt files inside `secrets` dir.


# Setup
1. Host machine needs to have `sops` installed, e.g. `brew install sops`.
2. Upload your pgp public key to one of the pgp key servers, e.g. pgp.mit.edu.
3. Create a PR with your pgp id added to `.sops.yaml`. To get your key id run `gpg --list-keys`.
```bash
gpg --list-keys
pub   rsa4096 2022-11-17 [SC]
      15AC8EA84FC2A5AE768FFD753CEBBA453DE5BCFD       <------- this is id
uid           [ultimate] Radosław Sporny <radoslaw.sporny@iohk.io>
sub   rsa4096 2022-11-17 [E]
```
4. Ask maintainers of the repo to rotate the secrets with your public key.
	1. [maintainer] follow [[#Rotating secrets]] steps.


# Edit secrets
For editing encrypted secrets we suggest VSCode with `signageos.signageos-vscode-sops` extension installed. It decrypts files automatically and allows for inline edits.


# Add new secrets
> **_DANGER:_**
>
> Uploading secrets in plain text exposes a security risk. Remember to always encrypt new secrets before you push them to the repo.

If you are using VSCode with `signageos.signageos-vscode-sops` extension, all new files inside `secrets` dir should be encrypted automatically. However, you may get an error if you don't have all PGP public keys listed in `.sops.yaml` imported to your keyring. [[#Import any missing public key to your keyring]] is the command that will import all missing PGP public keys, or you can do it one by one:
```bash
gpg --keyserver pgp.mit.edu --recv-keys <pgp key id>
```

If, for whatever reason, automatic encryption doesn't work you can always do it manually:
```bash
cd <project root dir>
sops -e -i <path to the new secret file>
```

> **_WARNING:_**
>
> Make sure to cd into project root so that `.sops.yaml` config file can be used, and all of us can decrypt the new file.


# Rotating secrets
##### Import any missing public key to your keyring
```bash
awk '/pgp: >-/{found=1; next} found && NF{gsub(/^[[:space:],]+|[[:space:],]+$/, ""); print}' .sops.yaml | xargs gpg --keyserver pgp.mit.edu --recv-keys
```

Note: if pgp.mit.edu doesn't work, check keyserver.ubuntu.com instead.

Decrypt all files
```bash
find secrets -type f -print0 | xargs -0 -I {} sops -d -i {}
```

Encrypt all files again
```bash
find secrets -type f -print0 | xargs -0 -I {} sops -e -i {}
```
