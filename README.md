# Enigma

Enigma fue una máquina utilizada durante la segunda guerra mundial para la encriptación de mensajes.
Fue una maravilla de la ingeniería en ese momento, pero en este proyecto, vamos a demostrar lo sencilla que era.

Obviamente no podemos sacarle el mérito al equipo de Inglaterra (en especial a Touring) que fue el que logró crackear el algoritmo de la misma, y sin siquiera saber como estaba conformada

## Tabla de contenidos
1. Como funcionaba
2. Como se ve reflejada en el código
3. Algún extra que voy a poner.


# Como funcionaba
Al menos la primer versión (Enigma 1) contaba con 3 rotores, un reflector, un plugboard y nada más.
Pero... ¿Como se comportaba cada mecanismo y que es lo que la hacía indescifrable?

Cada rotor tenía los números del 1 al 26 (por el contexto del alfabeto), pero contaba con un *notch*, que era una varilla arbitraria que forzaba a que en vez de el ciclo cortar en el 26, cortaba antes (era el número establecido arbitrario) De esta forma, ese patrón de 26 ciclos nunca se podía ver del todo reflejado, sumado a que eran 3 rotores de 5, donde tampoco se sabía de que modo estaban repartidos los números dentro de cada rotor, eso lo hacía más complejo, dando así un total de *26^3* combinaciones posibles.

Además, tenía otro componente como el *plugboard* que funcionaba para reemplazar (de forma simétrica) una letra, ejemplo, se conectaba la A a la G y de ese modo, cuando la máquina procesaba (antes de pasar a los rotores) si entraba una A y el plugboard estaba conectado a una G, el mismo circuito lo veía como una G, nunca se enteraba que venía una A.

Y por último, el último componente, el *reflector*, que se encargaba de asegurarse que si entraba una A y luego de todos los rotores, seguía apareciendo una A, lo reemplazaba por otra letra arbitraria (parecida al plugboard, pero el reflector no era configurable).

Una vez con todo eso armado, el proceso era muy sencillo y el siguiente:
letra -> plugboard.replace(letra) -> for rotor in rotors: rotors.forward(letra) -> reflector.reflect(letra) -> for rotor in rotors.reverse(): rotor.backward(letra) -> letra encriptada

Pero hay un detalle más, y es quizás el más importante de todos: el *stepping*. Cada vez que se presionaba una tecla, *antes* de que la señal arrancara a viajar por el circuito, el rotor de la derecha avanzaba una posición. Cuando ese rotor llegaba a su *notch*, en la siguiente pulsación arrastraba al rotor del medio, y lo mismo pasaba entre el del medio y el de la izquierda. Esto es lo que hacía que la misma letra ingresada dos veces seguidas no diera nunca el mismo resultado, y es lo que rompe cualquier intento de atacarla con un análisis de frecuencias clásico.

Hay además una particularidad bastante rara, el famoso *double stepping*: cuando el rotor del medio estaba en su notch, no sólo hacía avanzar al de la izquierda, sino que él mismo daba un paso extra en esa misma pulsación. Es un comportamiento que viene del diseño mecánico de los trinquetes, no de una decisión "lógica", pero está y hay que replicarlo si uno quiere ser fiel a la máquina original.

Y por último, ¿cómo se desencriptaba? Y acá está la magia: con la misma máquina, configurada igual (mismos rotores, mismas posiciones iniciales, mismo plugboard). Porque el reflector garantiza que el circuito sea *simétrico*, si A se transformaba en K, entonces K, en esa misma posición, se transforma en A. Mandabas el mensaje cifrado por la misma máquina y salía el original. Elegante, pero también una de las debilidades que terminaron usando para crackearla (una letra nunca podía cifrarse en sí misma).


# Como se ve reflejada en el código
Bueno, ahora la parte divertida. Vamos a ir mecanismo por mecanismo viendo cómo se traduce todo eso a Rust.

## El Rotor
El rotor es el componente con más estado, así que arranquemos por ahí.

```rust
struct Rotor {
    fwd: [usize; CONNECTIONS],
    bwd: [usize; CONNECTIONS],
    notch: usize,
    position: usize,
    ring: usize,
}
```

`fwd` y `bwd` son básicamente el cableado interno del rotor. `fwd[i]` te dice "si entra la señal por el cable `i`, sale por el cable `fwd[i]`", y `bwd` es lo mismo pero al revés (ya que la señal pasa dos veces por cada rotor, una de ida hacia el reflector y otra de vuelta). En vez de calcular la inversa cada vez, la precomputamos al construir el rotor:

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

Lo del `wiring` como string ("ekmflgdqvzntowyhxuspaibrcj" por ejemplo) es porque históricamente cada rotor venía con un cableado conocido (el rotor I, II, III, etc), y se suelen documentar como la permutación del alfabeto. Cada letra en esa cadena representa a qué letra del alfabeto se conecta esa posición.

Ahora, el truco está en el `forward` y `backward`, porque el rotor *rota*, entonces no podemos simplemente indexar `fwd[input]`:

```rust
fn forward(&self, input: usize) -> usize {
    let shifted = (input + self.position + CONNECTIONS - self.ring) % CONNECTIONS;
    let cross = self.fwd[shifted];
    (cross + CONNECTIONS + self.ring - self.position) % CONNECTIONS
}
```

Lo que hacemos es: primero corremos el input según la posición actual del rotor (y el ring setting, que es otra capa de offset configurable), después miramos por dónde sale en el cableado interno, y por último deshacemos ese corrimiento para devolver el resultado en el marco de referencia "absoluto". El `+ CONNECTIONS` está para evitar underflow al restar `usize`, que en Rust te pega un panic.

## El Plugboard
Este es el más simple de todos:

```rust
struct Plugboard {
    connections: [usize; CONNECTIONS],
}
```

Un array de 26 posiciones donde `connections[i] = j` significa que la letra `i` está conectada con la `j`. Por defecto cada letra está conectada consigo misma (la identidad), y al conectar dos letras lo hacemos *simétrico*:

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

Esa simetría es la clave: si A está enchufada con G, tanto buscar A devuelve G como buscar G devuelve A. El `if` de arriba evita que pisemos una conexión existente (si una letra ya estaba enchufada, no la tocamos).

## El Reflector
El reflector es conceptualmente parecido al plugboard pero no configurable y siempre conecta todas las letras de a pares:

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

Lo armamos a partir de 13 pares (26 / 2), y nos aseguramos de que ninguna letra esté conectada consigo misma. Esto último es lo que garantiza la propiedad de que *una letra nunca puede cifrarse en sí misma*, que como mencionaba antes, fue una de las debilidades aprovechadas para crackearla.

## El Stepping
Acá es donde entra el famoso *double stepping*:

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

Fijate que chequeamos los notches *antes* de avanzar nada. Si el del medio está en notch, avanza tanto el del medio como el de la izquierda (ese es el double stepping del rotor del medio). Si el de la derecha está en notch, arrastra al del medio. Y el de la derecha siempre avanza, en cada pulsación, sí o sí. El orden importa: por eso primero leemos el estado y después aplicamos los pasos.

## El flujo completo
Y finalmente, la función `encrypt` arma todo el recorrido que describimos en la parte 1:

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

Letra por letra: primero avanzamos los rotores, después la señal pasa por el plugboard, atraviesa los tres rotores hacia adelante, rebota en el reflector, vuelve hacia atrás por los tres rotores, pasa de nuevo por el plugboard y ahí tenemos nuestra letra encriptada. Tal cual como funcionaba la máquina física, pero en un par de líneas de Rust.
