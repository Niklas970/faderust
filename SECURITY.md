# Sicherheit

## Schwachstelle melden

Bitte **nicht** als öffentliches Issue. Nutze stattdessen auf GitHub unter
[Security → Report a vulnerability](https://github.com/Niklas970/faderust/security/advisories/new)
die private Meldung. Ich antworte in der Regel innerhalb weniger Tage.

*Please report vulnerabilities privately through GitHub's "Report a vulnerability" form, not as a public issue.*

## Unterstützte Versionen

Nur die neueste Version aus den [Releases](https://github.com/Niklas970/faderust/releases/latest) bekommt Korrekturen.

## Gut zu wissen

- Updates lädt die App nur von GitHub Releases und prüft vor der Installation eine Signatur (minisign). Das Setup selbst ist nicht mit einem Windows-Zertifikat signiert, deshalb warnt SmartScreen.
- Die Fernsteuerung für das Stream Deck lauscht nur auf `127.0.0.1:47811`, lehnt Browser-Anfragen ab und verlangt ein Token (`remote.token` im Konfigurationsordner, nur für den eigenen Windows-Benutzer lesbar). Programme desselben Benutzers können die Datei lesen und damit Fader und Presets steuern; andere Benutzer auf demselben PC nicht.
