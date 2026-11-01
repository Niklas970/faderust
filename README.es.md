<p align="center"><a href="README.md">English</a> · <a href="README.de.md">Deutsch</a> · <a href="README.fr.md">Français</a> · <b>Español</b> · <a href="README.it.md">Italiano</a> · <a href="README.nl.md">Nederlands</a> · <a href="README.pl.md">Polski</a></p>

<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Última versión"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="Licencia MIT"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078d4" alt="Windows 10 | 11">
  <img src="https://img.shields.io/badge/status-alpha-e2793f" alt="Alpha">
  <a href="https://github.com/Niklas970/faderust/releases"><img src="https://img.shields.io/github/downloads/Niklas970/faderust/total?label=Downloads" alt="Downloads"></a>
  <a href="https://github.com/Niklas970/faderust/stargazers"><img src="https://img.shields.io/github/stars/Niklas970/faderust?style=flat" alt="Stars"></a>
  <a href="https://github.com/Niklas970/faderust/commits/main"><img src="https://img.shields.io/github/last-commit/Niklas970/faderust" alt="Last commit"></a>
  <img src="https://img.shields.io/badge/Rust-stable-dea584?logo=rust&logoColor=white" alt="Rust">
</p>

<p align="center">
  Mezclador de audio para Windows, un sustituto ligero y abierto de Voicemeeter.<br>
  Cada salida tiene su propia mezcla, los dispositivos pueden ir y venir mientras funciona, y Faderust trae sus propios cables virtuales.
</p>

![Mesa de Faderust con dispositivos, programas y tres salidas](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust en 40 segundos"></a><br>
  <sub>Vista previa, <a href="assets/faderust-reel.mp4">vídeo en calidad completa</a> (para descargar)</sub>
</p>

> **Alfa.** Faderust funciona, pero apenas se ha probado en hardware real. Informa de los problemas como [issue](https://github.com/Niklas970/faderust/issues).

## Qué hace Faderust

- **Una mesa como Voicemeeter.** Cada dispositivo y cada programa tiene una tira con fader, MUTE y los botones A1, A2, A3 para las salidas. Cada salida tiene su propio fader master.
- **Matriz para el trabajo fino.** Como alternativa, en forma de tabla: cada entrada ajustada por separado en cada salida, p. ej. el micrófono más bajo en los auriculares que en el stream.
- **Captura programas directamente.** Discord, Spotify o el navegador tienen su propia tira con efectos, sin un cable virtual por programa. Faderust oculta los programas en silencio y los recupera uno a uno si quieres («N en silencio, mostrar»).
- **Cables virtuales propios, sin controlador adicional.** La aplicación configura por sí misma el «Faderust Kabel» (hasta cuatro), por ejemplo uno para el micrófono y otro para los programas. Detalles en [Faderust Kabel](#faderust-kabel).
- **Ordenado.** Las salidas se pueden ocultar con la × y vuelven solas en cuanto suena algo en ellas.
- **Los dispositivos van y vienen.** Apagar y encender unos auriculares Bluetooth, desconectar un dispositivo USB: Faderust se reconecta sin interrumpir los demás dispositivos.
- **Compatible con multipunto.** Faderust libera las salidas Bluetooth durante el silencio (a los 30 s, ajustable por salida) para que los auriculares puedan cambiar al móvil. Cuando vuelve el sonido, se abren solas.
- **Efectos por entrada:** supresión de ruido ([RNNoise](https://jmvalin.ca/demo/rnnoise/), contra teclado y ventilador), filtro paso alto, ecualizador de tres bandas y gate. La supresión de ruido funciona en dispositivos de 48 kHz y retrasa el sonido 10 ms.
- **Alemán e inglés.** El idioma sigue a Windows y se cambia en «Design».
- **Los ajustes se guardan solos.** El diálogo de dispositivos aplica los cambios poco después de la última entrada.
- **Autodiagnóstico.** En «Dispositivos», Faderust comprueba dispositivos, búferes, realimentación, control remoto y el plugin de Stream Deck y muestra qué está bien y qué no. El informe se puede copiar para un issue.
- **El sonido de Windows en un solo sitio:** dispositivos predeterminados para reproducción, grabación y llamadas, además del volumen por dispositivo.
- **Presets, inicio automático en la bandeja, actualizaciones en la aplicación** (con comprobación de firma) y un **plugin de Stream Deck**.
- **Compacto y en primer plano.** Dos botones en la barra de título: vista estrecha sin etiquetas y «Siempre visible», p. ej. junto a un juego. Ambos se recuerdan.
- **Tus propios colores:** color de acento y de señal a elegir, oscuro (negro OLED) o claro.

## Vistas

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Matriz: cada entrada ajustada por separado en cada salida"></td>
    <td><img src="assets/design.png" alt="Diálogo Design con vista, colores y apariencia"></td>
  </tr>
  <tr>
    <td><b>Matriz:</b> cada entrada en cada salida por separado, p. ej. el micrófono más bajo en los auriculares que en el stream.</td>
    <td><b>Design:</b> mesa o matriz, lado de las salidas, colores, oscuro o claro.</td>
  </tr>
</table>

## Instalación

1. Descarga `Faderust_*_x64-setup.exe` desde [Releases](https://github.com/Niklas970/faderust/releases/latest) y ejecútalo. El instalador no necesita permisos de administrador y habla alemán o inglés.
2. El instalador no está firmado, por eso Windows SmartScreen avisa: «Más información» → «Ejecutar de todas formas».
3. Abre Faderust desde el menú Inicio. La aplicación avisa de las versiones nuevas arriba, en la barra de título; un clic en el número de versión busca al instante.

Requisito: Windows 10 (versión 2004 o posterior) o Windows 11.

## Primeros pasos

1. Abre **Dispositivos** y crea entradas (micrófono, línea, Faderust Kabel) y salidas (auriculares, altavoces, stream). Bluetooth necesita unos 80 ms de búfer, USB normalmente de 30 a 60 ms.
2. En la **mesa**, elige con A1–A3 dónde suena una entrada. El fader regula juntas todas las salidas activadas.
3. Manejar los faders: arrastrar, rueda del ratón (más fino con Mayús), doble clic para 0 dB, o hacer clic en el número de dB de debajo y escribir un valor.
4. **Efectos** bajo una tira abre filtro paso alto, ecualizador y gate.
5. En **Design** se cambia entre mesa y matriz y se elige el lado de las salidas y los colores.

### Programas

**+ Programa** en la mesa muestra todos los programas que están reproduciendo sonido ahora. **+ Entrada** captura uno directamente. Para que no lo oigas dos veces, Faderust pasa primero el programa a un Faderust Kabel libre que no escuchas (si está configurado) y luego lo captura. Un programa que está en silencio lo oculta Faderust; **N en silencio, mostrar** lo recupera.

### Micrófono para Discord, Teams y compañía

En **Dispositivos**, **+ Micrófono de programa** crea una salida en un Faderust Kabel libre que lleva solo el micrófono con sus efectos. En el programa, elige como micrófono el lado de grabación del mismo cable. Faderust silencia las realimentaciones (una salida suena en el cable del que viene una entrada) y muestra «Bucle».

### Faderust Kabel

El Faderust Kabel es un dispositivo de sonido USB virtual (estéreo, 48 kHz, 24 bits) que proporciona el propio Faderust. Faderust lo muestra como «Faderust Kabel (Reproducción)» y «Faderust Kabel (Grabación)»; lo que un programa reproduce en el lado de reproducción sale de nuevo por el lado de grabación.

1. En **Dispositivos** → **Faderust Kabel**, haz clic en **Configurar** (una sola vez, con una solicitud de administrador). Faderust descarga [usbip-win2](https://github.com/vadimgrn/usbip-win2), comprueba la suma de verificación y lo instala en segundo plano. No aparece aparte en la lista de aplicaciones de Windows.
2. Activa **Faderust Kabel** y elige la cantidad (1 a 4), por ejemplo uno para el micrófono y otro para los programas. Windows los muestra en sus ajustes como «Faderust Cable», «Faderust Cable 2», etc.
3. **Quitar** deshace la configuración, incluido usbip-win2.

Nota: el cable sigue siendo un experimento y se ha probado en poco hardware.

## Límites

- Los programas recuerdan faders, salidas y efectos mientras su tira siga en la mesa: si Discord se reinicia, vuelve con sus ajustes anteriores. Si quitas la tira en «Dispositivos», se pierden.
- Sin controlador de kernel propio. Los cables virtuales («Faderust Kabel», hasta cuatro) son dispositivos de sonido USB que Faderust proporciona por USB/IP. Windows usa para ello su controlador de audio USB integrado; la parte USB/IP es [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, firmado por Microsoft), que la configuración instala una sola vez.
- Windows entrega el sonido por programa, no por pestaña del navegador ni aplicación web. Todo lo de un navegador pasa por el mismo proceso de audio.
- Tras una liberación, Bluetooth tarda alrededor de un segundo en volver a dar sonido. Quien no lo quiera, pone la salida en «Nunca liberar».
- La asignación «programa → dispositivo» usa la misma interfaz de Windows no documentada que el mezclador de volumen de los ajustes.

Ajustes: `%APPDATA%\io.github.niklas970.faderust\config.toml`, junto a él el registro `faderust.log`, que ayuda con los errores.

## Desarrollo

Rust para el núcleo de audio y la interfaz ([iced](https://iced.rs), sin WebView2).

| Ruta | Contenido |
|---|---|
| `crates/engine` | Núcleo de audio: dispositivos (WASAPI vía cpal), búferes circulares, compensación de deriva, matriz de mezcla, efectos, captura de programas |
| `crates/usbcable` | Faderust Kabel: dispositivo de audio USB virtual sobre USB/IP |
| `crates/core` | Lógica detrás de la interfaz: configuración, dispositivos, programas, cables, control remoto, actualizador |
| `crates/gui` | El programa `faderust`: ventana iced, bandeja, inicio automático |
| `streamdeck` | Plugin de Stream Deck |

Requisitos: Rust (stable); Node.js para el plugin de Stream Deck.

```sh
cargo test
cargo run --release -p faderust
```

- Construir el instalador: `makensis` con `installer/faderust.nsi` (los parámetros están al inicio del archivo). Las releases las genera el workflow «Release».
- Motor sin interfaz: `cargo run --release -p faderust-engine --example mixer -- --list`

Antes de un commit: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Las dependencias nuevas necesitan una licencia permisiva (`cargo deny check`).

## Stream Deck

El plugin en [`streamdeck/`](streamdeck/README.md) controla Faderust desde el Elgato Stream Deck: faders con dial o tecla, silenciar, presets y dispositivo predeterminado de Windows. Haz doble clic en `com.niklas970.faderust.streamDeckPlugin` desde [Releases](https://github.com/Niklas970/faderust/releases) (Stream Deck 7.1 o posterior). A partir de ahí, el instalador incluye el plugin: en «Dispositivos», Faderust muestra si hay una versión más reciente del plugin y la abre con un clic en el software de Stream Deck, que solo pide confirmación.

## Licencia

[MIT](LICENSE)
