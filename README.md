# diceodds

A command-line tool that answers one question: given a dice notation
expression, what is the exact probability of each possible outcome?

Dice notation ("2d6+3", "1d20-1d4") looks simple but there's no single
agreed grammar for it. Different tools accept different things: spaces
around operators, uppercase `D`, leading zeros, omitted counts (`d6`
meaning `1d6`). If you're pasting an expression from a rulebook or a
forum post into a tool, you usually want to know whether what you typed
is actually well-formed, not have the tool silently guess what you meant.

`diceodds` is strict by default. It rejects whitespace, uppercase `D`,
missing signs between terms, and leading zeros, and tells you exactly
what it didn't like and where. If you want it to be forgiving instead,
pass `--lenient`.

The probabilities are computed exactly, by convolving the uniform
distribution of each die, not estimated by simulation.

## Usage

Full probability table for a sum of dice plus a modifier:

```
$ diceodds "2d6+3"
   value  probability
       5     2.7778%
       6     5.5556%
       7     8.3333%
       8    11.1111%
       9    13.8889%
      10    13.8889%
      11    11.1111%
      12     8.3333%
      13     5.5556%
      14     2.7778%
      15     2.7778%

min: 5  max: 15  mean: 10.00
```

Ask a specific question instead of printing the whole table:

```
$ diceodds "1d20" --at-least 15
P(total >= 15) = 30.0000%
```

Roll four dice and keep the best three:

```
$ diceodds "4d6kh3" --exact 18
P(total == 18) = 1.6204%
```

Subtraction works too, as its own signed term:

```
$ diceodds "2d6-1d4" --exact 0
P(total == 0) = 6.9444%
```

Strict mode rejects sloppy input by default:

```
$ diceodds "2D6 + 3"
error: unexpected whitespace at position 3 (use --lenient to allow it)
```

`--lenient` accepts it anyway:

```
$ diceodds "2D6 + 3" --lenient
   value  probability
       5     2.7778%
       ...
```

## Notation supported

- `NdS`: roll N dice with S sides each and sum them.
- `NdSkhK` keeps the highest K dice and `NdSklK` keeps the lowest K, so
  `4d6kh3` is the usual ability score roll and `2d20kl1` is
  disadvantage. K must be between 1 and N. Because kept dice depend on
  each other, these terms are computed differently and have a tighter
  size limit than plain dice; oversized ones are rejected with an error.
- A bare number is a flat modifier.
- Terms are combined with `+` and `-`, e.g. `2d6+1d4-2`.
- Strict mode requires: no whitespace, lowercase `d`, an explicit count
  before `d` (`1d6`, not `d6`), no leading zeros, and an explicit `+` or
  `-` before every term after the first, and a count after `kh`/`kl`.
- `--lenient` relaxes all of the above: whitespace is stripped, `D`, `KH`
  and `KL` are accepted, a missing dice count or keep count defaults to 1,
  and leading zeros are allowed.
- Regardless of mode, an expression is capped at 32 terms, 500 dice
  total, and 1000 sides per die, to keep the computation exact and fast.

## Building

Standard library only, no external crates:

```
cargo build --release
```

## License

MIT, see LICENSE.
