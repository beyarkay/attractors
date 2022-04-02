# Attractors
A [Rust](https://www.rust-lang.org/) project to generate and explore [strange
attractors](https://en.wikipedia.org/wiki/Attractor#Strange_attractor) visually
and easily. The emphasis is on using strange attractors as an artistic medium,
and not explicitly on the maths behind them (although the maths is
unavoidable).

See below for example images of strange attractors to give you a feel for what
they look like. After that is some info about getting started with the project
and running the code to find your own. Following that is some more background
info about strange attractors and the different types.

## Examples of strange attractors
See more images in the [`imgs`](https://github.com/beyarkay/attractors/tree/main/img)
directory. They're all very high resolution (about 7000 pixels on each side) so
you can print them out at An size at 300 DPI.

![](img/15.png)
![](img/11_small.png)

## Get started and making your own strange attractors

Install the rust compiler and cargo by following the instructions at [install
rust](https://www.rust-lang.org/tools/install) or by running the command: 
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then clone this project: 

```sh
git clone https://github.com/beyarkay/attractors.git
```

Change directory and run the project in `--release` mode:
```sh
cd attractors && cargo run --release
```
The extra optimisations given by the `--release` flag are _really_ required for
a smooth experience.

There will be some console output explaining the available commands and what
they do, and two windows should pop up: a small diagnostics window (which you
can ignore) and a larger square window.

Click on the square window to focus it. The attractor will automatically be
drawn, and you can change the 4 parameters fed to the attractor (named a, b, c,
d) with vim-like key bindings:
```
j -> decrease a; k -> increase a; 
J -> decrease b; K -> increase b;
h -> decrease c; l -> increase c; 
H -> decrease d; L -> increase d;
```

Additionally, you can select a random set of parameters by pressing R:
```
`[R]` => Randomize the Clifford parameters and re-run the attractor with these new parameters
```

You can `print` off an attractor by pressing `p` which will save it as a `.png`
in `cached/clifford/`.
```
`[P]` => Save the attractor in high resolution to disc as png (enabled: true)
```

You can also change the hue of the attractor by pressing `e` or `E`:
```
`[E]` => Increase or decrease the LCH hue intercept by 0.01 (enabled: true)
```

## What are Strange Attractors
Strange attractors are (usually) a recursive formula which take in a point in
2D or 3D space and (using a set of parameters) return a different point in that
space. An example equation might be:
```
x_new = sin(a * y_old) + c * cos(a * x_old)
y_new = sin(b * x_old) + d * cos(b * y_old)
```

That new point is then sent back into the same recursive formula to generate
another point. And another, and another, etc. Those points are then coerced
into a pixel grid, and the resulting 2D or 3D histogram sometimes looks
spectacular.

See these links for examples of strange attractors.

- [Clifford Attractors](http://paulbourke.net/fractals/clifford/)
- [De Jong Attractors](http://paulbourke.net/fractals/peterdejong/)
- [Lyapunov Exponent Attractors](http://paulbourke.net/fractals/lyapunov/)
- [Sprott Polynomial Attractors](http://paulbourke.net/fractals/sprott/)
- [Juan Attractors](http://paulbourke.net/fractals/juan2/)
- [Den Tsucs Attractors](http://paulbourke.net/fractals/tsucs/)
- [Sánchez 'Bad Hairday' Attractors](http://paulbourke.net/fractals/2dmap/), also see
[here](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
- [Arneodo Attractors](http://paulbourke.net/fractals/arneodo/)
- [Catrián Attractors](http://paulbourke.net/fractals/JuanCatrian/)
- [Burke-Shaw Attractors](http://paulbourke.net/fractals/burkeshaw/)
- [Yu-Wang Attractors](http://paulbourke.net/fractals/yuwang/)

