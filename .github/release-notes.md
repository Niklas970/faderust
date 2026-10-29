## Neu in 0.5.5

- **Klarere Meldung beim Kabel:** Lässt sich ein virtuelles Kabel (VB-Cable) nicht öffnen, nennt die Meldung jetzt die passenden Ursachen (hängender Windows-Audiodienst nach dem Ruhemodus, zweite faderust.exe) statt des Hinweises auf Bluetooth-Kopfhörer
- **Weniger Last durch das Fenster:** Die Oberfläche zeichnet nicht mehr bei jedem Pegel-Update (20-mal pro Sekunde) alle Streifen und Listen neu, sondern nur die Pegel-Balken. Das senkt die Prozessorlast von WebView2 spürbar

## Installation

`Faderust_*_x64-setup.exe` laden und starten (keine Admin-Rechte nötig). SmartScreen warnt, weil das Setup nicht signiert ist: „Weitere Informationen“ → „Trotzdem ausführen“. Wer Faderust schon hat, bekommt das Update direkt in der App.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, oder nach dem Update in Faderust unter „Geräte“ auf „Plugin aktualisieren“ klicken.

Alle Funktionen und eine Anleitung stehen im [README](https://github.com/Niklas970/faderust#readme). Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.
