<h1 align="center">
    <br>
    <!-- <img --> 
    <!--   src="GITHUB LINK OF BREAKER LOGO" -->
    <!--   alt="Breaker" -->
    <!--   width="200"> -->
    <!-- <br> -->
    Breaker
    <br>
</h1>

<h4 align="center">
    A minimal audio livecoding language written in Rust.
</h4>

<p align="center">
    <a href="https://crates.io/crates/breakers"><img alt="Crates.io Version" src="https://img.shields.io/crates/v/breakers"></a>
    <a href="https://docs.rs/breakers/latest/breakers/"><img alt="docs.rs" src="https://img.shields.io/docsrs/breakers"></a>
</p>

<p align="center">
    <a href="#key-features">Key Features</a> •
    <a href="#how-to-use">How To Use</a> •
    <a href="#roadmap">Roadmap</a> •
    <a href="#license">License</a>
</p>

![screenshot](https://github.com/mielpeeters/breaker/assets/72082402/e179248c-c9f0-4d9e-90ac-7cb46d190eb9)

## Key Features

- grids: configurable sequence of tokens which loop
    - token: either a note, a chord, a repeat `_`, a pause `&`, or an integer (which can be mapped to a sample)
    - gridtokens' length is configurable (`note grid_name 3/4` for example)
- mapping: custom token integers can be mapped to samples, with optional probability parameter
- mixing: different grids can be mixed
- synths: currently, only one default synth sound (filtered square) is supported
    - in the future, configurable synths will be added

## Roadmap

- 

## How To Use

Here is an example breaker file, `my_first_beat.br`: 

```breaker
tempo 120 4/4

// make a new grid and name it 'beat'
// note: _ is a pause, & means 'continue playing'
grid beat {
    1 _ 2 _ 3 &
    1 _ 2 _ 3 4
} 
// map the symbols in the grid to samples
// note: 40% means that 4 will play with probability 40%
map beat {
    1: kick,
    2: hihat,
    3: snare,
    4: hihat2 40%,
}
// length of one token in the grid (default: 1/16)
note beat 1/16

grid chords {
    Cm7/C & & _
    [3]AbM7/Ab & & _
    [3]Fm7/F & & _
    [3]Fm7/F & & _
}

note chords 1/8

grid bassline {
    [2]c__[2]c__[2]c_
    [1]ab__[1]ab__[1]ab_
    [1]f__[1]f__[1]f_
    [1]f__[1]f__[1]f_
}

// adjust mix (default: 1.0)
mix bassline 2.0
mix beat 1.2
```

We can run this file using this command:
```shell
breaker -s samples/ my_first_beat.br
```

## License

GNU GPLv3
