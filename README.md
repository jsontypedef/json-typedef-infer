# jtd-infer

`jtd-infer` generates a JSON Typedef schema from example data.

For high-level guidance on how to use this package, see ["Inferring a JSON
Typedef Schema from Real Data"][jtd-jtd-infer] in the JSON Typedef docs.

## Installation

To install `jtd-infer`, you have a few options:

### Install with Homebrew

This option is recommended if you're on macOS.

```bash
brew install jsontypedef/jsontypedef/jtd-infer
```

### Install with Docker

This option is recommended on non-Mac platforms, or if you're running
`jtd-infer` in some sort of script and you want to make sure that everyone
running the script uses the same version of `jtd-infer`.

```bash
docker pull jsontypedef/jtd-tools
```

If you opt to use the Docker approach, you will need to change all invocations
of `jtd-infer` in this README from:

```bash
jtd-infer [...]
```

To:

```bash
docker exec -it jsontypedef/jtd-tools /jtd-infer [...]
```

### Install with Cargo

This option is recommended if you already have `cargo` installed, or if you
would prefer to use a version of `jtd-infer` compiled on your machine:

```bash
cargo install jtd-infer
```

## Usage

> See the top of this README for links to high-level guidance on how to use
> `jtd-infer`.

To invoke `jtd-infer`, you can either:

1. Have it read from STDIN. This is the default behavior.
2. Have it read from a file. To do this, pass a file name as the last argument
   to `jtd-infer`.

`jtd-infer` can read a _sequence_ of JSON messages. So for example, if you have
a file like this in `data.json`:

```json
{ "name": "john doe", "age": 42 }
{ "name": "jane doe", "age": 45 }
```

You can give it to `jtd-infer` in two ways:

```bash
# Both of these do the same thing.
cat data.json | jtd-infer
jtd-infer data.json
```

In both cases, you'd get this output:

```json
{"properties":{"name":{"type":"string"},"age":{"type":"uint8"}}}
```

## Advanced Usage: Providing Hints

By default, `jtd-infer` will never output `enum`, `values`, or `discriminator`
schemas. This is by design: by always being consistent with what it outputs,
`jtd-infer` is more predictable and reliable.

If you want `jtd-infer` to output an `enum`, `values`, or `discriminator`, you
can use the `--enum-hint`, `--values-hint`, and `--discriminator-hint` flags.
You can pass each of these flags multiple times.

All of the hint flags accept [JSON
Pointers](https://tools.ietf.org/html/rfc6901) as values. If you're used to the
JavaScript-y syntax of referring to things as `$.foo.bar`, the equivalent JSON
Pointer is `/foo/bar`. `jtd-infer` treats `-` as a "wildcard". `/foo/-/bar` is
equivalent to the JavaScript-y `$.foo.*.bar`.

As a corner-case, if you want to point to the *root* / top-level of your input,
then use the empty string as the path. See ["Using
`--values-hint`"](##using---values-hint) for an example of this.

### Using `--enum-hint`

By default, strings are always inferred to be `{ "type": "string" }`:

```bash
echo '["foo", "bar", "baz"]' | jtd-infer
```

```json
{"elements":{"type":"string"}}
```

But you can instead have `jtd-infer` output an enum by providing a path to the
string you consider to be an enum. In this case, it's any element of the root of
the array -- the JSON Pointer for that is `/-`:

```bash
echo '["foo", "bar", "baz"]' | jtd-infer --enum-hint=/-
```

```json
{"elements":{"enum":["bar","baz","foo"]}}
```

### Using `--values-hint`

By default, objects are always assumed to be "structs", and `jtd-infer` will
generate `properties` / `optionalProperties`. For example:

```bash
echo '{"x": [1, 2, 3], "y": [4, 5, 6], "z": [7, 8, 9]}' | jtd-infer
```

```json
{"properties":{"y":{"elements":{"type":"uint8"}},"z":{"elements":{"type":"uint8"}},"x":{"elements":{"type":"uint8"}}}}
```

If your data is more like a map / dictionary, pass a `values-hint` that points
to the object that you want a `values` schema from. In this case, that's the
root-level object, which in JSON Pointer is just an empty string:

```bash
echo '{"x": [1, 2, 3], "y": [4, 5, 6], "z": [7, 8, 9]}' | jtd-infer --values-hint=
```

```json
{"values":{"elements":{"type":"uint8"}}}
```

### Using `--discriminator-hint`

By default, objects are always assumed to be "structs", and `jtd-infer` will
generate `properties` / `optionalProperties`. For example:

```bash
echo '[{"type": "s", "value": "foo"},{"type": "n", "value": 3.14}]' | jtd-infer
```

```json
{"elements":{"properties":{"value":{},"type":{"type":"string"}}}}
```

If your data has a special "type" property that tells you what's in the rest of
the object, then use `--discriminator-hint` to point to that property.
`jtd-infer` will output an appropriate `discriminator` schema instead:

```bash
echo '[{"type": "s", "value": "foo"},{"type": "n", "value": 3.14}]' | jtd-infer --discriminator-hint=/-/type | jq
```

```json
{
  "elements": {
    "discriminator": "type",
    "mapping": {
      "s": {
        "properties": {
          "value": {
            "type": "string"
          }
        }
      },
      "n": {
        "properties": {
          "value": {
            "type": "float64"
          }
        }
      }
    }
  }
}
```

[jtd-jtd-infer]: https://jsontypedef.com/docs/tools/jtd-infer
