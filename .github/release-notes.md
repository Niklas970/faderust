## Neu in 0.5.3

- **LED-Balken im Gleichtakt:** Die Eingangs-Balken werden um die Pufferzeit der Ausgänge verzögert angezeigt. Ein Eingang und der Ausgang, der ihn spielt, leuchten dadurch gleichzeitig statt nacheilend
- **Protokoll unter „Geräte“:** Der Protokoll-Knopf sitzt jetzt im Geräte-Dialog neben dem Selbsttest, die Leiste unten entfällt
- **Geräte-Dialog ohne Programme:** Bei den Eingängen erscheinen nur noch echte Geräte, Programme bleiben im Programm-Dialog
- **Programme ohne Ton:** Faderust nimmt jetzt auch Prozesse eines Programms auf, die erst nach dem Start der Aufnahme erscheinen (zweites Fenster, neu gestarteter Audioprozess). Vorher blieb so ein Programm stumm, obwohl der Eingang aktiv war
- **Nach dem Ruhemodus:** Nach dem Aufwachen baut Faderust alle Geräte und Programm-Aufnahmen neu auf. Vorher blieben sie oft scheinbar verbunden, aber tot: Programme blieben stumm, nur das Mikro ging, und der Prozessor lief mit 12 % Last
- **Schneller aufwachen:** Freigegebene Ausgänge achten alle 10 statt 50 ms auf zurückkehrenden Ton. Die Dauer des Aufwachens steht im Protokoll („wake took … ms“)

## Installation

`Faderust_*_x64-setup.exe` laden und starten (keine Admin-Rechte nötig). SmartScreen warnt, weil das Setup nicht signiert ist: „Weitere Informationen“ → „Trotzdem ausführen“. Wer Faderust schon hat, bekommt das Update direkt in der App.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, oder nach dem Update in Faderust unter „Geräte“ auf „Plugin aktualisieren“ klicken.

Alle Funktionen und eine Anleitung stehen im [README](https://github.com/Niklas970/faderust#readme). Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.
