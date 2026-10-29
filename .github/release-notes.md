## Neu in 0.5.6

- **LED-Balken sparen Prozessor:** Die Balken werden nicht mehr bei jeder Bewegung neu gezeichnet, nur noch ein Deckblatt wird verschoben. Das senkt die Lastspitzen beim Sprechen oder bei Musik, wenn sich viele Balken gleichzeitig bewegen
- **Weniger Last für den Windows-Audiodienst:** Die Liste der Programme mit Ton wird nur noch alle 2 Sekunden abgefragt statt für jeden wartenden Programm-Eingang jede Sekunde. Das entlastet den Windows-Audiodienst, der bei zu vielen Abfragen hängen kann („Fehler bei der Audiowiedergabe“ im Browser)

## Installation

`Faderust_*_x64-setup.exe` laden und starten (keine Admin-Rechte nötig). SmartScreen warnt, weil das Setup nicht signiert ist: „Weitere Informationen“ → „Trotzdem ausführen“. Wer Faderust schon hat, bekommt das Update direkt in der App.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, oder nach dem Update in Faderust unter „Geräte“ auf „Plugin aktualisieren“ klicken.

Alle Funktionen und eine Anleitung stehen im [README](https://github.com/Niklas970/faderust#readme). Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.
