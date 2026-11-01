<p align="center"><a href="README.md">Deutsch</a> · <a href="README.en.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <b>Nederlands</b> · <a href="README.pl.md">Polski</a></p>

<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Audiomixer voor Windows, een compact, open alternatief voor Voicemeeter.<br>
  Elke uitgang krijgt een eigen mix, apparaten mogen tijdens het gebruik komen en gaan, en Faderust brengt zijn eigen virtuele kabels mee.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Nieuwste versie"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="MIT-licentie"></a>
</p>

![Faderust-paneel met apparaten, programma’s en drie uitgangen](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust in 40 seconden"></a><br>
  <sub>Voorbeeld, <a href="assets/faderust-reel.mp4">video in volle kwaliteit</a> (om te downloaden)</sub>
</p>

> **Alfa.** Faderust werkt, maar is nog nauwelijks op echte hardware getest. Meld problemen graag als [issue](https://github.com/Niklas970/faderust/issues).

## Wat Faderust kan

- **Een paneel zoals Voicemeeter.** Elk apparaat en elk programma krijgt een kanaal met fader, MUTE en de knoppen A1, A2, A3 voor de uitgangen. Elke uitgang heeft een eigen masterfader.
- **Matrix voor het fijnwerk.** Als alternatief als tabel: elke ingang afzonderlijk geregeld op elke uitgang, bijvoorbeeld de microfoon zachter op de koptelefoon dan in de stream.
- **Programma’s direct opnemen.** Discord, Spotify of de browser krijgen een eigen kanaal met effecten, zonder virtuele kabel per programma. Stille programma’s verbergt Faderust en haalt ze op verzoek een voor een terug (“N stil, tonen”).
- **Eigen virtuele kabels, geen extra stuurprogramma.** De app richt de “Faderust Kabel” (tot vier) zelf in, bijvoorbeeld een voor de microfoon en een voor programma’s. Details onder [Faderust Kabel](#faderust-kabel).
- **Opgeruimd.** Uitgangen zijn met de × te verbergen en komen vanzelf terug zodra er weer geluid op speelt.
- **Apparaten komen en gaan.** Bluetooth-koptelefoon uit en weer aan, USB-apparaat loskoppelen: Faderust verbindt opnieuw zonder de andere apparaten te onderbreken.
- **Multipoint-vriendelijk.** Bluetooth-uitgangen geeft Faderust bij stilte vrij (na 30 s, per uitgang instelbaar), zodat de koptelefoon naar de telefoon kan wisselen. Komt er weer geluid, dan openen ze vanzelf.
- **Effecten per ingang:** ruisonderdrukking ([RNNoise](https://jmvalin.ca/demo/rnnoise/), tegen toetsenbord en ventilator), low-cut, driebands-EQ en gate. De ruisonderdrukking werkt bij apparaten met 48 kHz en vertraagt het geluid met 10 ms.
- **Duits en Engels.** De taal volgt Windows en is te wijzigen onder “Design”.
- **Instellingen worden vanzelf opgeslagen.** Het apparatendialoog neemt wijzigingen kort na de laatste invoer over.
- **Zelftest.** Onder “Apparaten” controleert Faderust apparaten, buffers, terugkoppeling, afstandsbediening en Stream Deck-plugin en toont wat in orde is en wat niet. Het rapport is te kopiëren voor een issue.
- **Windows-geluid op één plek:** standaardapparaten voor afspelen, opnemen en gesprekken, plus het volume per apparaat.
- **Presets, automatisch starten in het systeemvak, updates in de app** (met handtekeningcontrole) en een **Stream Deck-plugin**.
- **Compact en op de voorgrond.** Twee knoppen in de titelbalk: smalle weergave zonder labels en “Altijd bovenaan”, bijvoorbeeld naast een spel. Beide blijven onthouden.
- **Eigen kleuren:** accent- en signaalkleur vrij te kiezen, donker (OLED-zwart) of licht.

## Weergaven

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Matrix: elke ingang afzonderlijk geregeld op elke uitgang"></td>
    <td><img src="assets/design.png" alt="Design-dialoog met weergave, kleuren en uiterlijk"></td>
  </tr>
  <tr>
    <td><b>Matrix:</b> elke ingang op elke uitgang afzonderlijk, bijvoorbeeld de microfoon zachter op de koptelefoon dan in de stream.</td>
    <td><b>Design:</b> paneel of matrix, kant van de uitgangen, kleuren, donker of licht.</td>
  </tr>
</table>

## Installatie

1. Download `Faderust_*_x64-setup.exe` onder [Releases](https://github.com/Niklas970/faderust/releases/latest) en start het. De installer heeft geen beheerdersrechten nodig en spreekt Duits of Engels.
2. De installer is niet ondertekend, daarom waarschuwt Windows SmartScreen: “Meer informatie” → “Toch uitvoeren”.
3. Open Faderust via het Startmenu. De app meldt nieuwe versies bovenaan de titelbalk; een klik op het versienummer zoekt meteen.

Vereiste: Windows 10 (vanaf versie 2004) of Windows 11.

## Aan de slag

1. Open **Apparaten** en maak ingangen (microfoon, line-in, Faderust Kabel) en uitgangen (koptelefoon, luidsprekers, stream) aan. Bluetooth heeft ongeveer 80 ms buffer nodig, USB meestal 30 tot 60 ms.
2. Kies in het **paneel** met A1–A3 waar een ingang naartoe speelt. De fader regelt alle ingeschakelde uitgangen samen.
3. Faders bedienen: slepen, muiswiel (fijner met Shift), dubbelklik voor 0 dB, of klik op het dB-getal eronder en typ een waarde.
4. **Effecten** onder een kanaal opent low-cut, EQ en gate.
5. Onder **Design** wissel je tussen paneel en matrix en kies je de kant van de uitgangen en de kleuren.

### Programma’s

**+ Programma** in het paneel toont alle programma’s die nu geluid afspelen. **+ Ingang** neemt er een direct op. Zodat je het niet dubbel hoort, zet Faderust het programma eerst op een vrije Faderust Kabel waar je niet naar luistert (indien ingericht) en neemt het dan op. Een programma dat op dat moment stil is, verbergt Faderust; **N stil, tonen** haalt het terug.

### Microfoon voor Discord, Teams en co.

Onder **Apparaten** maakt **+ Programmamicrofoon** een uitgang op een vrije Faderust Kabel aan waarop alleen de microfoon met zijn effecten ligt. Kies in het programma de opnamekant van dezelfde kabel als microfoon. Terugkoppeling (een uitgang speelt op de kabel waar een ingang van komt) dempt Faderust en toont “Lus”.

### Faderust Kabel

De Faderust Kabel is een virtueel USB-geluidsapparaat (stereo, 48 kHz, 24 bit) dat Faderust zelf levert. Faderust toont het als “Faderust Kabel (Afspelen)” en “Faderust Kabel (Opnemen)”; wat een programma op de afspeelkant speelt, komt aan de opnamekant weer naar buiten.

1. Klik onder **Apparaten** → **Faderust Kabel** op **Inrichten** (eenmalig, met een beheerdersvraag). Faderust downloadt [usbip-win2](https://github.com/vadimgrn/usbip-win2), controleert de controlesom en installeert het op de achtergrond. In de app-lijst van Windows verschijnt het niet apart.
2. Schakel **Faderust Kabel** in en kies het aantal (1 tot 4), bijvoorbeeld een voor de microfoon en een voor programma’s. Windows toont ze in zijn instellingen als “Faderust Cable”, “Faderust Cable 2” enzovoort.
3. **Verwijderen** maakt de inrichting weer ongedaan, inclusief usbip-win2.

Let op: de kabel is nog een experiment en op weinig hardware getest.

## Beperkingen

- Programma’s onthouden faders, uitgangen en effecten zolang hun kanaal in het paneel blijft: start Discord opnieuw, dan komt het terug met zijn oude instellingen. Verwijder je het kanaal onder “Apparaten”, dan zijn ze weg.
- Geen eigen kernelstuurprogramma. De virtuele kabels (“Faderust Kabel”, tot vier) zijn USB-geluidsapparaten die Faderust zelf via USB/IP levert. Windows gebruikt daarvoor zijn ingebouwde USB-audiostuurprogramma; het USB/IP-deel is [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, door Microsoft ondertekend), dat de inrichting eenmalig installeert.
- Windows levert geluid per programma, niet per browsertabblad of webapp. Alles uit één browser loopt via hetzelfde audioproces.
- Bluetooth heeft na een vrijgave ongeveer een seconde nodig voordat er weer geluid is. Wie dat niet wil, zet de uitgang op “Nooit vrijgeven”.
- De koppeling “programma → apparaat” gebruikt dezelfde ongedocumenteerde Windows-interface als de volumemixer in de instellingen.

Instellingen: `%APPDATA%\io.github.niklas970.faderust\config.toml`, ernaast het logboek `faderust.log`, dat bij fouten helpt.

## Ontwikkeling

Rust voor de audiokern en de interface ([iced](https://iced.rs), geen WebView2).

| Pad | Inhoud |
|---|---|
| `crates/engine` | Audiokern: apparaten (WASAPI via cpal), ringbuffers, driftcompensatie, mixmatrix, effecten, programma-opname |
| `crates/usbcable` | Faderust Kabel: virtueel USB-audioapparaat via USB/IP |
| `crates/core` | Logica achter de interface: configuratie, apparaten, programma’s, kabels, afstandsbediening, updater |
| `crates/gui` | Het programma `faderust`: iced-venster, systeemvak, automatisch starten |
| `streamdeck` | Stream Deck-plugin |

Vereisten: Rust (stable); Node.js voor de Stream Deck-plugin.

```sh
cargo test
cargo run --release -p faderust
```

- Setup bouwen: `makensis` met `installer/faderust.nsi` (parameters staan bovenaan het bestand). Releases ontstaan via de workflow “Release”.
- Engine zonder interface: `cargo run --release -p faderust-engine --example mixer -- --list`

Voor een commit: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Nieuwe afhankelijkheden hebben een permissieve licentie nodig (`cargo deny check`).

## Stream Deck

De plugin in [`streamdeck/`](streamdeck/README.md) bedient Faderust vanaf de Elgato Stream Deck: faders met draaiknop of toets, dempen, presets en het Windows-standaardapparaat. Dubbelklik op `com.niklas970.faderust.streamDeckPlugin` uit de [Releases](https://github.com/Niklas970/faderust/releases) (Stream Deck 7.1 of nieuwer). Daarna levert de setup de plugin mee: onder “Apparaten” toont Faderust of er een nieuwere pluginversie klaarstaat en opent die met een klik in de Stream Deck-software, die dan alleen om bevestiging vraagt.

## Licentie

[MIT](LICENSE)
