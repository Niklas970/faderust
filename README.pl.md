<p align="center"><a href="README.md">English</a> · <a href="README.de.md">Deutsch</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <a href="README.nl.md">Nederlands</a> · <b>Polski</b></p>

<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Mikser audio dla Windows, lekki i otwarty zamiennik Voicemeetera.<br>
  Każde wyjście ma własny miks, urządzenia mogą się pojawiać i znikać podczas pracy, a Faderust ma własne kable wirtualne.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Najnowsza wersja"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="Licencja MIT"></a>
</p>

![Pulpit Faderust z urządzeniami, programami i trzema wyjściami](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust w 40 sekund"></a><br>
  <sub>Podgląd, <a href="assets/faderust-reel.mp4">film w pełnej jakości</a> (do pobrania)</sub>
</p>

> **Alfa.** Faderust działa, ale prawie nie był jeszcze testowany na prawdziwym sprzęcie. Błędy zgłaszaj jako [issue](https://github.com/Niklas970/faderust/issues).

## Co potrafi Faderust

- **Pulpit jak w Voicemeeterze.** Każde urządzenie i każdy program ma kanał z faderem, MUTE i przyciskami A1, A2, A3 dla wyjść. Każde wyjście ma własny fader główny.
- **Macierz do precyzyjnej pracy.** Alternatywnie jako tabela: każde wejście regulowane osobno na każdym wyjściu, np. mikrofon ciszej w słuchawkach niż w streamie.
- **Bezpośrednie przechwytywanie programów.** Discord, Spotify lub przeglądarka dostają własny kanał z efektami, bez wirtualnego kabla dla każdego programu. Ciche programy Faderust ukrywa i na życzenie przywraca pojedynczo („N cichych, pokaż”).
- **Własne kable wirtualne, bez dodatkowego sterownika.** Aplikacja sama konfiguruje „Faderust Kabel” (do czterech), np. jeden dla mikrofonu i jeden dla programów. Szczegóły w [Faderust Kabel](#faderust-kabel).
- **Porządek.** Wyjścia można ukryć przyciskiem ×, a wracają same, gdy tylko znów zagra na nich dźwięk.
- **Urządzenia przychodzą i odchodzą.** Wyłączenie i włączenie słuchawek Bluetooth, odłączenie urządzenia USB: Faderust łączy się ponownie, nie przerywając pozostałych urządzeń.
- **Przyjazny dla multipoint.** Faderust zwalnia wyjścia Bluetooth podczas ciszy (po 30 s, ustawiane dla każdego wyjścia), aby słuchawki mogły przełączyć się na telefon. Gdy dźwięk wraca, otwierają się same.
- **Efekty dla każdego wejścia:** redukcja szumów ([RNNoise](https://jmvalin.ca/demo/rnnoise/), przeciw klawiaturze i wentylatorowi), filtr górnoprzepustowy, trzypasmowy korektor i bramka szumów. Redukcja szumów działa na urządzeniach 48 kHz i opóźnia dźwięk o 10 ms.
- **Niemiecki i angielski.** Język podąża za Windows i można go zmienić w „Design”.
- **Ustawienia zapisują się same.** Okno urządzeń zatwierdza zmiany krótko po ostatnim wpisie.
- **Autotest.** W „Urządzenia” Faderust sprawdza urządzenia, bufory, sprzężenie zwrotne, zdalne sterowanie i wtyczkę Stream Deck i pokazuje, co jest w porządku, a co nie. Raport można skopiować do issue.
- **Dźwięk Windows w jednym miejscu:** urządzenia domyślne dla odtwarzania, nagrywania i rozmów oraz głośność każdego urządzenia.
- **Presety, autostart w zasobniku, aktualizacje w aplikacji** (z weryfikacją podpisu) i **wtyczka Stream Deck**.
- **Kompaktowy i na wierzchu.** Dwa przyciski na pasku tytułu: wąski widok bez podpisów i „Zawsze na wierzchu”, np. obok gry. Oba ustawienia są zapamiętywane.
- **Własne kolory:** kolor akcentu i sygnału do wyboru, ciemny (czerń OLED) lub jasny.

## Widoki

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Macierz: każde wejście regulowane osobno na każdym wyjściu"></td>
    <td><img src="assets/design.png" alt="Okno Design z widokiem, kolorami i wyglądem"></td>
  </tr>
  <tr>
    <td><b>Macierz:</b> każde wejście na każdym wyjściu osobno, np. mikrofon ciszej w słuchawkach niż w streamie.</td>
    <td><b>Design:</b> pulpit lub macierz, strona wyjść, kolory, ciemny lub jasny.</td>
  </tr>
</table>

## Instalacja

1. Pobierz `Faderust_*_x64-setup.exe` z [Releases](https://github.com/Niklas970/faderust/releases/latest) i uruchom. Instalator nie wymaga uprawnień administratora i mówi po niemiecku lub angielsku.
2. Instalator nie jest podpisany, dlatego Windows SmartScreen ostrzega: „Więcej informacji” → „Uruchom mimo to”.
3. Otwórz Faderust z menu Start. Aplikacja informuje o nowych wersjach u góry paska tytułu; kliknięcie numeru wersji od razu ich szuka.

Wymagania: Windows 10 (od wersji 2004) lub Windows 11.

## Pierwsze kroki

1. Otwórz **Urządzenia** i utwórz wejścia (mikrofon, line-in, Faderust Kabel) oraz wyjścia (słuchawki, głośniki, stream). Bluetooth potrzebuje około 80 ms bufora, USB zwykle 30 do 60 ms.
2. Na **pulpicie** wybierz przyciskami A1–A3, gdzie gra dane wejście. Fader reguluje wszystkie włączone wyjścia razem.
3. Obsługa faderów: przeciąganie, kółko myszy (dokładniej z Shift), dwukrotne kliknięcie dla 0 dB albo kliknięcie liczby dB poniżej i wpisanie wartości.
4. **Efekty** pod kanałem otwierają filtr górnoprzepustowy, korektor i bramkę.
5. W **Design** przełączasz się między pulpitem a macierzą i wybierasz stronę wyjść oraz kolory.

### Programy

**+ Program** na pulpicie pokazuje wszystkie programy, które właśnie odtwarzają dźwięk. **+ Wejście** przechwytuje jeden bezpośrednio. Aby nie słyszeć go podwójnie, Faderust najpierw przełącza program na wolny Faderust Kabel, którego nie słuchasz (jeśli skonfigurowany), a potem go przechwytuje. Program, który w tej chwili milczy, Faderust ukrywa; **N cichych, pokaż** go przywraca.

### Mikrofon dla Discorda, Teams i spółki

W **Urządzenia** przycisk **+ Mikrofon programu** tworzy wyjście na wolnym Faderust Kabel, na którym jest tylko mikrofon z jego efektami. W programie wybierz jako mikrofon stronę nagrywania tego samego kabla. Sprzężenia zwrotne (wyjście gra na kabel, z którego pochodzi wejście) Faderust wycisza i pokazuje „Pętla”.

### Faderust Kabel

Faderust Kabel to wirtualne urządzenie dźwiękowe USB (stereo, 48 kHz, 24 bity), które Faderust udostępnia sam. Faderust pokazuje je jako „Faderust Kabel (Odtwarzanie)” i „Faderust Kabel (Nagrywanie)”; to, co program zagra na stronie odtwarzania, wychodzi z powrotem po stronie nagrywania.

1. W **Urządzenia** → **Faderust Kabel** kliknij **Skonfiguruj** (jednorazowo, z pytaniem o uprawnienia administratora). Faderust pobiera [usbip-win2](https://github.com/vadimgrn/usbip-win2), sprawdza sumę kontrolną i instaluje go w tle. Na liście aplikacji Windows nie pojawia się osobno.
2. Włącz **Faderust Kabel** i wybierz liczbę (1 do 4), np. jeden dla mikrofonu i jeden dla programów. Windows pokazuje je w ustawieniach jako „Faderust Cable”, „Faderust Cable 2” itd.
3. **Usuń** cofa konfigurację razem z usbip-win2.

Uwaga: kabel to wciąż eksperyment i był testowany na niewielu urządzeniach.

## Ograniczenia

- Programy zapamiętują fadery, wyjścia i efekty, dopóki ich kanał pozostaje na pulpicie: po ponownym uruchomieniu Discord wraca ze starymi ustawieniami. Jeśli usuniesz kanał w „Urządzenia”, ustawienia przepadają.
- Brak własnego sterownika jądra. Kable wirtualne („Faderust Kabel”, do czterech) to urządzenia dźwiękowe USB, które Faderust udostępnia przez USB/IP. Windows używa do tego wbudowanego sterownika audio USB; część USB/IP to [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, podpisany przez Microsoft), który konfiguracja instaluje jednorazowo.
- Windows dostarcza dźwięk według programu, a nie karty przeglądarki czy aplikacji webowej. Wszystko z jednej przeglądarki przechodzi przez ten sam proces audio.
- Po zwolnieniu Bluetooth potrzebuje około sekundy, zanim wróci dźwięk. Kto tego nie chce, ustawia wyjście na „Nigdy nie zwalniaj”.
- Przypisanie „program → urządzenie” korzysta z tego samego nieudokumentowanego interfejsu Windows co mikser głośności w ustawieniach.

Ustawienia: `%APPDATA%\io.github.niklas970.faderust\config.toml`, obok dziennik `faderust.log`, który pomaga przy błędach.

## Rozwój

Rust dla rdzenia audio i interfejsu ([iced](https://iced.rs), bez WebView2).

| Ścieżka | Zawartość |
|---|---|
| `crates/engine` | Rdzeń audio: urządzenia (WASAPI przez cpal), bufory pierścieniowe, kompensacja dryfu, macierz miksu, efekty, przechwytywanie programów |
| `crates/usbcable` | Faderust Kabel: wirtualne urządzenie audio USB przez USB/IP |
| `crates/core` | Logika za interfejsem: konfiguracja, urządzenia, programy, kable, zdalne sterowanie, aktualizacje |
| `crates/gui` | Program `faderust`: okno iced, zasobnik, autostart |
| `streamdeck` | Wtyczka Stream Deck |

Wymagania: Rust (stable); Node.js dla wtyczki Stream Deck.

```sh
cargo test
cargo run --release -p faderust
```

- Budowanie instalatora: `makensis` z `installer/faderust.nsi` (parametry są na początku pliku). Wydania powstają przez workflow „Release”.
- Silnik bez interfejsu: `cargo run --release -p faderust-engine --example mixer -- --list`

Przed commitem: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Nowe zależności wymagają liberalnej licencji (`cargo deny check`).

## Stream Deck

Wtyczka w [`streamdeck/`](streamdeck/README.md) steruje Faderustem z Elgato Stream Deck: fadery pokrętłem lub przyciskiem, wyciszanie, presety i domyślne urządzenie Windows. Kliknij dwukrotnie `com.niklas970.faderust.streamDeckPlugin` z [Releases](https://github.com/Niklas970/faderust/releases) (Stream Deck 7.1 lub nowszy). Od tej pory instalator dołącza wtyczkę: w „Urządzenia” Faderust pokazuje, czy dostępna jest nowsza wersja wtyczki, i otwiera ją jednym kliknięciem w oprogramowaniu Stream Deck, które prosi już tylko o potwierdzenie.

## Licencja

[MIT](LICENSE)
