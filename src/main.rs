use std::collections::HashMap;

const CONNECTIONS: usize = 26;
#[derive(Debug)]
struct Rotor {
    fwd: [usize; CONNECTIONS],
    bwd: [usize; CONNECTIONS],
    notch: usize,
    position: usize,
    ring: usize,
}

#[derive(Debug)]
struct Plugboard {
    connections: [usize; CONNECTIONS],
}

#[derive(Debug)]
struct Reflector {
    state: HashMap<usize, usize>,
}

impl Reflector {
    fn new(inputs: &[(usize, usize); CONNECTIONS / 2]) -> Self {
        let mut state = HashMap::new();
        for (i, o) in inputs {
            if i == o {
                panic!("The 'wires' can't be wired by themself")
            }
            // Bidirectional reflection, i <> o
            state.insert(*i, *o);
            state.insert(*o, *i);
        }
        Self { state }
    }

    fn reflect(&self, input: usize) -> usize {
        self.state[&input]
    }
}

impl Plugboard {
    fn connect(mut self, a: usize, b: usize) -> Self {
        if self.connections[a as usize] != a || self.connections[b as usize] != b {
            return self;
        };
        self.connections[a as usize] = b;
        self.connections[b as usize] = a;
        self
    }
    fn get(&self, n: usize) -> usize {
        self.connections[n as usize]
    }
}

#[derive(Debug)]
struct Machine {
    rotors: [Rotor; 3],
    reflector: Reflector,
    plugboard: Plugboard,
}

fn char_to_idx(idx: char) -> usize {
    idx as usize - b'a' as usize
}

impl Rotor {
    fn new(wiring: &str, notch: usize, letter: char) -> Self {
        let wiring = wiring.chars().collect::<Vec<_>>();
        let mut fwd = [0; CONNECTIONS];
        let mut bwd = [0; CONNECTIONS];
        for i in 0..CONNECTIONS {
            fwd[i] = char_to_idx(wiring[i]);
            bwd[fwd[i]] = i;
        }
        Rotor {
            fwd,
            bwd,
            ring: char_to_idx(letter),
            notch,
            position: 0,
        }
    }

    fn at_notch(&self) -> bool {
        self.notch == self.position
    }

    fn step(&mut self) {
        self.position = (self.position + 1) % CONNECTIONS;
    }

    fn set_position(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    fn forward(&self, input: usize) -> usize {
        let shifted = (input + self.position + CONNECTIONS - self.ring) % CONNECTIONS;
        let cross = self.fwd[shifted];
        (cross + CONNECTIONS + self.ring - self.position) % CONNECTIONS
    }

    fn backward(&self, input: usize) -> usize {
        let shifted = (input + self.position + CONNECTIONS - self.ring) % CONNECTIONS;
        let cross = self.bwd[shifted];
        (cross + CONNECTIONS + self.ring - self.position) % CONNECTIONS
    }
}

impl Machine {
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
}

fn main() {
    let msg = "hello";
    let result = encrypt_message(msg);
    println!("{msg} -> {result}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity() {
        let a = "ekmflgdqvzntowyhxuspaibrcj";
        let b = "vbyoqhewtxricksmdzpunalgjf";
        assert_ne!(
            a.chars().map(char_to_idx).collect::<Vec<_>>(),
            b.chars().map(char_to_idx).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn plugboard() {
        let plugboard = Plugboard {
            connections: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25,
            ],
        }
        .connect(15, 13)
        .connect(14, 20);
        // A <-> B
        assert_eq!(plugboard.get(plugboard.get(15)), 15);
        assert_eq!(plugboard.get(1), 1);
    }

    #[test]
    fn rotors() {
        let wiring = "ekmflgdqvzntowyhxuspaibrcj";
        let mut rotor = Rotor::new(wiring, 16, 'a').set_position(0);
        // (0 + 0 + 26 - 0) % 26 = 0;
        assert_eq!(rotor.forward(0), 4);
        assert_eq!(wiring.as_bytes()[rotor.forward(0)] as char, 'l');
        rotor.step();
        // (0 + 1 + 26 - 16) % 26 = 11;
        // (10 + 26 + 0 - 1) % 26 = 9;
        assert_eq!(rotor.forward(0), 9);
    }

    #[test]
    fn rotors_displaced() {
        let wiring = "vbyoqhewtxricksmdzpunalgjf";
        let mut rotor = Rotor::new(wiring, 16, 'a').set_position(6);
        // (25 + 6 + 26 - 0) % 26 = 5;
        // (7 + 26 + 0 - 6) % 26 = 1;
        assert_eq!(rotor.forward(25), 1);
        assert_eq!(wiring.as_bytes()[rotor.forward(25)] as char, 'b');
        rotor.step();
        // (25 + 7 + 26 - 0) % 26 = 6;
        // (4 + 26 + 0 - 7) % 26 = 22;
        assert_eq!(rotor.forward(25), 23);
        assert_eq!(wiring.as_bytes()[rotor.forward(25)] as char, 'g');
    }

    #[test]
    fn full_rotor_flow() {
        let wiring = "ekmflgdqvzntowyhxuspaibrcj";
        let rotor = Rotor::new(wiring, 16, 'a').set_position(5);

        for i in 0..26 {
            assert_eq!(rotor.backward(rotor.forward(i)), i);
        }
    }

    #[test]
    fn good_reflector() {
        let pairs = [
            ('a', 'q'),
            ('b', 'm'),
            ('c', 'z'),
            ('d', 'x'),
            ('e', 'y'),
            ('f', 'v'),
            ('g', 'u'),
            ('h', 't'),
            ('i', 's'),
            ('k', 'r'),
            ('l', 'p'),
            ('n', 'w'),
            ('o', 'j'),
        ]
        .map(|(a, b)| (char_to_idx(a), char_to_idx(b)));
        let reflector = Reflector::new(&pairs);
        let output = reflector.reflect(0);
        assert_eq!(output, char_to_idx('q'));
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let message = "helloworld";
        let same = message;

        let cipher = encrypt_message(message);
        let cipher2 = encrypt_message(same);

        assert_ne!(cipher, cipher2);

        let decrypted = encrypt_message(&cipher);
        let decrypted2 = encrypt_message(&cipher2);

        assert_eq!(decrypted, message);
        assert_eq!(decrypted2, same);
        assert_eq!(decrypted, decrypted2);
    }
}
fn encrypt_message(message: &str) -> String {
    let plugboard = Plugboard {
        connections: [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25,
        ],
    }
    .connect(0, 6) // A <> G
    .connect(2, 9); // C <> J

    let rotors = [
        Rotor::new("ekmflgdqvzntowyhxuspaibrcj", 16, 'a').set_position(0), // rotor I
        Rotor::new("ajdksiruxblhwtmcqgznpyfvoe", 4, 'a').set_position(0),  // rotor II
        Rotor::new("bdfhjlcprtxvznyeiwgakmusqo", 21, 'a').set_position(0), // rotor III
    ];

    let inputs = [
        ('y', 'r'),
        ('u', 'h'),
        ('q', 's'),
        ('l', 'd'),
        ('p', 'x'),
        ('n', 'g'),
        ('o', 'k'),
        ('m', 'i'),
        ('e', 'b'),
        ('f', 'z'),
        ('c', 'w'),
        ('v', 'j'),
        ('a', 't'),
    ]
    .map(|(a, b)| (char_to_idx(a), char_to_idx(b)));
    let reflector = Reflector::new(&inputs); // UKW-B

    let mut machine = Machine {
        plugboard,
        rotors,
        reflector,
    };
    machine.encrypt(message)
}
