## Neu in 0.6.4

- **USBip-Eintrag verschwindet auch bei bestehender Einrichtung:** Wer das Faderust Kabel schon vor 0.6.3 eingerichtet hat, sah usbip-win2 weiter als eigene App in Windows. Faderust erkennt das jetzt und bietet unter „Geräte“ → „Faderust Kabel“ die Einrichtung noch einmal an (eine Administrator-Abfrage), danach ist der Eintrag ausgeblendet
- **Geräte-Einstellungen speichern von selbst:** Änderungen im Dialog „Geräte“ werden kurz nach der letzten Eingabe übernommen, der Speichern-Knopf entfällt (es bleibt „Schließen“)
- **Klarere Namen für das Faderust Kabel:** statt „Lautsprecher (Faderust Cable)“ steht in der Oberfläche „Faderust Kabel (Wiedergabe)“ bzw. „(Aufnahme)“, bei mehreren mit Nummer. Das Kabel meldet Windows außerdem die Namen „Playback“ und „Recording“ (Versuch, ob Windows sie statt „Lautsprecher/Mikrofon“ übernimmt)

## Installation

`Faderust_*_x64-setup.exe` laden und starten (keine Admin-Rechte nötig). SmartScreen warnt, weil das Setup nicht signiert ist: „Weitere Informationen“ → „Trotzdem ausführen“. Wer Faderust schon hat, bekommt das Update direkt in der App.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, oder nach dem Update in Faderust unter „Geräte“ auf „Plugin aktualisieren“ klicken.

Alle Funktionen und eine Anleitung stehen im [README](https://github.com/Niklas970/faderust#readme). Fehler bitte als [Issue](https://github.com/Niklas970/faderust/issues) melden.
