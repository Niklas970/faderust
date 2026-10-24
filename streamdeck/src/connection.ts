import { Faderust } from "./faderust";

/** The one connection all actions share. `FADERUST_URL` is for tests. */
export const faderust = new Faderust(process.env.FADERUST_URL || undefined);
