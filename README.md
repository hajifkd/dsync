# dsync

A git-like command to manually manipulate Dropbox remote files & directories.

## Usage

Currently, `clone`, `pull`, `add`, `init` is supported. `push` and `rm` are to be supported. 

`.gitignore` like ignore file, `.dsyncignore`, is supported.

### `clone`

```sh
dsync clone /hoge
```

### `pull`

```sh
cd CLONED_DIR
dsync pull
```

### `add`

```sh
dsync add SOME_FILE_OR_DIR
```

### `init`

```sh
dsync init LOCAL_DIR REMOTE_DIR
```

### Syntax of `.dsyncignore`

Currently, only a file named `.dsyncignore` at the repo root is supported.

You can see examples at [the test code](src/ignore.rs). Roughly speaking, 

* A line staring with # is regarded as a comment line.
* A line starting with / matches the relative path from the root directory.
* A line not starting with / matches the any path containing the sequence.
* \*\* matches to any series of directories.
* \* matches any number of character except for /
* ? matches any charactor.

TODO need to determine whether is_ignored matches local/remote path.

# document

https://www.dropbox.com/developers/documentation/http/documentation

# TODO

* Use PKCE? I feel PKCE is not invulnerable either for this type of application. 