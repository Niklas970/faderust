// The engine sends its state 20 times a second, almost always with only the
// level meters changed. Replacing the whole state each time makes every
// strip and list redraw; this keeps the state object and updates the levels
// in place unless something else changed.

const withoutLevel = ({ levelDb, ...rest }) => rest;

/** Everything in `s` except the meter levels and the lane statistics nobody shows. */
export function structureKey(s) {
  return JSON.stringify([
    s.inputs.map(withoutLevel),
    s.outputs.map(withoutLevel),
    s.gains,
    s.lanes.map((row) => row.map((l) => l.looped)),
  ]);
}

/**
 * Returns the state to show: `current` with new levels when only levels
 * changed, otherwise `next`. `keyOf` remembers the last key.
 */
export function createSync() {
  let key = "";
  return (current, next) => {
    const nextKey = structureKey(next);
    if (current && nextKey === key) {
      next.inputs.forEach((d, i) => (current.inputs[i].levelDb = d.levelDb));
      next.outputs.forEach((d, o) => (current.outputs[o].levelDb = d.levelDb));
      return current;
    }
    key = nextKey;
    return next;
  };
}
