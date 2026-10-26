import streamDeck from "@elgato/streamdeck";

import { Device } from "./actions/device";
import { Master } from "./actions/master";
import { Mute } from "./actions/mute";
import { Preset } from "./actions/preset";
import { Strip } from "./actions/strip";
import { Volume } from "./actions/volume";
import { faderust } from "./connection";

faderust.on("error", (message) => streamDeck.logger.warn(`Faderust: ${message}`));

streamDeck.actions.registerAction(new Strip());
streamDeck.actions.registerAction(new Master());
streamDeck.actions.registerAction(new Volume());
streamDeck.actions.registerAction(new Mute());
streamDeck.actions.registerAction(new Preset());
streamDeck.actions.registerAction(new Device());

faderust.version = streamDeck.info.plugin.version;
faderust.connect();
streamDeck.connect();
