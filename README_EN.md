# Enigma

Enigma was a machine used during World War II to encrypt messages.
It was a marvel of engineering at the time, but in this project, we're going to show just how simple it actually was.

Obviously we can't take credit away from the British team (especially Turing), who were the ones that managed to crack its algorithm, and without even knowing how it was put together.

## Table of contents
1. How it worked
2. How it's reflected in the code
3. Some extra stuff I'll be adding.


# How it worked
At least the first version (Enigma 1) had 3 rotors, a reflector, a plugboard and nothing else.
But... how did each mechanism behave and what was it that made it unbreakable?

Each rotor had the numbers 1 to 26 on it (because of the alphabet context), but it had a *notch*, which was an arbitrary pin that forced the cycle to break before reaching 26 instead of at 26 (it was set arbitrarily). That way, the 26-cycle pattern could never be fully seen, and on top of that there were 3 rotors out of 5, where you also didn't know how the numbers were laid out inside each rotor, and that made it more complex, giving a total of *26^3* possible combinations.

It also had another component, the *plugboard*, which worked by swapping (symmetrically) one letter for another, for example, A was wired to G, and that way, when the machine processed a letter (before going into the rotors), if an A came in and the plugboard had it connected to a G, the circuit itself just saw a G, it never knew an A had come in.

And lastly, the last component, the *reflector*, which made sure that if an A came in and after going through all the rotors it was still coming out as an A, it would replace it with another arbitrary letter (similar to the plugboard, but the reflector wasn't configurable).

Once you had all of that put together, the process was very simple and went like this:
letter -> plugboard.replace(letter) -> for rotor in rotors: rotors.forward(letter) -> reflector.reflect(letter) -> for rotor in rotors.reverse(): rotor.backward(letter) -> encrypted letter

But there's one more detail, and it's probably the most important one of all: the *stepping*. Every time a key was pressed, *before* the signal started traveling through the circuit, the rightmost rotor would advance one position. When that rotor hit its *notch*, on the next keystroke it would drag the middle rotor along with it, and the same thing happened between the middle one and the left one. This is what made it so that the same letter typed twice in a row would never give the same result, and it's what breaks any attempt at attacking it with a classic frequency analysis.

There's also a pretty weird quirk, the famous *double stepping*: when the middle rotor was at its notch, it didn't only push the left one forward, it also took an extra step itself on that same keystroke. It's a behavior that comes from the mechanical design of the pawls, not from a "logical" decision, but it's there and you have to replicate it if you want to be faithful to the original machine.

And finally, how was a message decrypted? And here's the magic: with the same machine, configured the same way (same rotors, same starting positions, same plugboard). Because the reflector guarantees the circuit is *symmetric*, if A was turned into K, then K, in that same position, gets turned into A. You'd send the encrypted message through the same machine and out came the original. Elegant, but also one of the weaknesses they ended up using to crack it (a letter could never be encrypted as itself).


# How it's reflected in the code
Alright, now the fun part. We're going to go mechanism by mechanism and see how all of that translates into Rust.

## The Rotor
The rotor is the component with the most state, so let's start there.

```rust
struct Rotor {
    fwd: [usize; CONNECTIONS],
    bwd: [usize; CONNECTIONS],
    notch: usize,
    position: usize,
    ring: usize,
}
```

`fwd` and `bwd` are basically the internal wiring of the rotor. `fwd[i]` tells you "if the signal comes in through wire `i`, it goes out through wire `fwd[i]`", and `bwd` is the same but in reverse (since the signal goes through each rotor twice, once on the way to the reflector and once on the way back). Instead of computing the inverse every time, we precompute it when building the rotor:

```rust
fn new(wiring: &str, notch: usize, letter: char) -> Self {
    let wiring = wiring.chars().collect::<Vec<_>>();
    let mut fwd = [0; CONNECTIONS];
    let mut bwd = [0; CONNECTIONS];
    for i in 0..CONNECTIONS {
        fwd[i] = char_to_idx(wiring[i]);
        bwd[fwd[i]] = i;
    }
    ...
}
```

The `wiring` as a string ("ekmflgdqvzntowyhxuspaibrcj" for example) is because historically each rotor came with a known wiring (rotor I, II, III, etc.), and they're usually documented as a permutation of the alphabet. Each letter in that string represents which letter of the alphabet that position is connected to.

Now, the trick is in `forward` and `backward`, because the rotor *rotates*, so we can't just index `fwd[input]`:

```rust
fn forward(&self, input: usize) -> usize {
    let shifted = (input + self.position + CONNECTIONS - self.ring) % CONNECTIONS;
    let cross = self.fwd[shifted];
    (cross + CONNECTIONS + self.ring - self.position) % CONNECTIONS
}
```

What we do is: first we shift the input according to the rotor's current position (and the ring setting, which is another configurable offset layer), then we look up where it comes out in the internal wiring, and finally we undo that shift to return the result in the "absolute" frame of reference. The `+ CONNECTIONS` is there to avoid underflow when subtracting `usize`, which in Rust would panic on you.

## The Plugboard
This one is the simplest of all:

```rust
struct Plugboard {
    connections: [usize; CONNECTIONS],
}
```

An array of 26 positions where `connections[i] = j` means letter `i` is connected to letter `j`. By default each letter is connected to itself (the identity), and when we connect two letters we do it *symmetrically*:

```rust
fn connect(mut self, a: usize, b: usize) -> Self {
    if self.connections[a as usize] != a || self.connections[b as usize] != b {
        return self;
    };
    self.connections[a as usize] = b;
    self.connections[b as usize] = a;
    self
}
```

That symmetry is the key: if A is plugged into G, looking up A returns G and looking up G also returns A. The `if` at the top prevents us from overwriting an existing connection (if a letter was already plugged in, we don't touch it).

## The Reflector
The reflector is conceptually similar to the plugboard but it's not configurable and always connects all letters in pairs:

```rust
struct Reflector {
    state: HashMap<usize, usize>,
}

fn new(inputs: &[(usize, usize); CONNECTIONS / 2]) -> Self {
    let mut state = HashMap::new();
    for (i, o) in inputs {
        if i == o {
            panic!("The 'wires' can't be wired by themself")
        }
        state.insert(*i, *o);
        state.insert(*o, *i);
    }
    Self { state }
}
```

We build it from 13 pairs (26 / 2), and we make sure no letter is connected to itself. That last bit is what guarantees the property that *a letter can never be encrypted as itself*, which as I mentioned earlier, was one of the weaknesses they leveraged to crack it.

## The Stepping
This is where the famous *double stepping* comes in:

```rust
fn step_rotors(&mut self) {
    let [left, mid, right] = &mut self.rotors;

    let mid_at_notch = mid.at_notch();
    let right_at_notch = right.at_notch();

    if mid_at_notch {
        mid.step();
        left.step();
    }
    if right_at_notch {
        mid.step();
    }

    right.step();
}
```

Notice that we check the notches *before* advancing anything. If the middle one is at its notch, both the middle one and the left one advance (that's the middle rotor's double stepping). If the right one is at its notch, it drags the middle one along. And the right one always advances, on every keystroke, no matter what. The order matters: that's why we read the state first and apply the steps after.

## The full flow
And finally, the `encrypt` function puts together the whole path we described in part 1:

```rust
pub fn encrypt(&mut self, message: &str) -> String {
    let mut buffer = String::new();
    for letter in message.chars() {
        self.step_rotors();

        let mut signal = self.plugboard.get(char_to_idx(letter));

        for rotor in self.rotors.iter() {
            signal = rotor.forward(signal);
        }

        signal = self.reflector.reflect(signal);

        for rotor in self.rotors.iter().rev() {
            signal = rotor.backward(signal);
        }

        signal = self.plugboard.get(signal);

        buffer.push((signal as u8 + b'a') as char);
    }
    buffer
}
```

Letter by letter: first we step the rotors, then the signal goes through the plugboard, crosses the three rotors going forward, bounces off the reflector, comes back through the three rotors, goes through the plugboard again, and there we have our encrypted letter. Exactly how the physical machine worked, but in a couple of lines of Rust.
