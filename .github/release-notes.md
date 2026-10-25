## Neu in 0.5.0

- **Rauschunterdrückung** pro Eingang (RNNoise): filtert Tastatur, Lüfter und Rauschen aus dem Mikro. Läuft bei Geräten mit 48 kHz und verzögert den Ton um 10 ms
- **Deutsch und Englisch:** Die Oberfläche folgt der Windows-Sprache, umstellbar unter „Design“
- **Selbsttest** unter „Geräte“: prüft Geräte, Puffer, Rückkopplung, Fernsteuerung und Stream-Deck-Plugin; der Bericht lässt sich für ein Issue kopieren
- **Stream-Deck-Plugin aus der App aktualisieren:** Das Setup bringt das Plugin mit, unter „Geräte“ siehst du, ob eine neuere Version bereitliegt
- **Fernsteuerung mit Token:** Das Stream Deck meldet sich mit einem Token aus dem Konfigurationsordner an. **Das Plugin muss dafür einmal aktualisiert werden** (Plugin 0.3.1), ältere Plugins verbinden sich nicht mehr
- **Kompakte Ansicht und „Immer im Vordergrund“:** zwei Knöpfe in der Titelleiste
- **Fenster passt sich an:** Kommt ein Streifen dazu oder fällt einer weg, ändert Faderust die Fensterbreite selbst (nicht bei maximiertem Fenster)
- **Schmalere Programm-Streifen:** ohne Skala, mit kleineren A-Knöpfen
- **Latenz nach einem Aussetzer:** Staut sich nach einem Aussetzer des Ausgangs Ton an, wird er verworfen statt mit bis zu einer halben Sekunde Verzögerung gespielt
- Sicherheit: schärfere Rechte der Workflows, `SECURITY.md`, Dependabot

## Installation

`Faderust_*_x64-setup.exe` laden und starten (keine Admin-Rechte nötig). SmartScreen warnt, weil das Setup nicht signiert ist: „Weitere Informationen“ → „Trotzdem ausführen“. Wer Faderust schon hat, bekommt das Update direkt in der App.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, oder nach dem Update in Faderust unter „Geräte“ auf „Plugin aktualisieren“ klicken.

Alle Funktionen und eine Anleitung stehen im [README](https://github.com/Niklas970/faderust#readme). Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.
