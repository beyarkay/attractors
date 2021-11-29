# Attractors

### Generate, visualise, and explore strange attractor visually

This repository is a work in progress, and will contain Rust code to:

- generate strange attractors of various kinds,
- visualise those attractors,
- save images of the attractors,
- create videos of the attractors, as one or more parameters vary,
- and to 'explore' the parameter space of the attractor

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

## (Planned) Structure
Generating the attractors takes a while, since often over 5 million points need
to be calculated for a semi-decent image to make itself visible, and each
generated point depends on the point that was generated before it, meaning that
the operation can not be made concurrent.

As the user explores the parameter space, the generated attractors should be
cached as a text file of points, so that the attractor can be quickly retrieved
if the user happens on that location in parameter space again. The name of the
file should fully encapsulate the parameter set, so that files don't have to be
opened in order to find the correct attractor.

## Installation

As this is unfinished, installation is not recommended. But if you must, then
you'll have to install Rust, and then

1. `git clone https://github.com/beyarkay/attractors.git`
2. `cargo doc --open` to open up the documentation (it's comprehensive), or
3. `cargo run` to execute the main body of the code.

There's no guarantees (yet) of it working or even doing something you want it
to do though.

# TODO

- Add images of attractors
- Implement visualisations
- Implement different attractor types

