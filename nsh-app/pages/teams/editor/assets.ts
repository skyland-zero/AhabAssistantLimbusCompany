// The editor's art: one status-effect plate per gift system and one portrait
// per sinner. Imports are static so the bundle owns the files; nothing reads a
// checkout path at runtime.
import type { SystemName } from "../../../teams-mirror";
import bleed from "../../../assets/status_effects/bleed.png";
import blunt from "../../../assets/status_effects/blunt.png";
import burn from "../../../assets/status_effects/burn.png";
import charge from "../../../assets/status_effects/charge.png";
import pierce from "../../../assets/status_effects/pierce.png";
import poise from "../../../assets/status_effects/poise.png";
import rupture from "../../../assets/status_effects/rupture.png";
import sinking from "../../../assets/status_effects/sinking.png";
import slash from "../../../assets/status_effects/slash.png";
import tremor from "../../../assets/status_effects/tremor.png";
import donQuixote from "../../../assets/sinners/don_quixote.png";
import faust from "../../../assets/sinners/faust.png";
import gregor from "../../../assets/sinners/gregor.png";
import heathcliff from "../../../assets/sinners/heathcliff.png";
import hongLu from "../../../assets/sinners/hong_lu.png";
import ishmael from "../../../assets/sinners/ishmael.png";
import meursault from "../../../assets/sinners/meursault.png";
import outis from "../../../assets/sinners/outis.png";
import rodion from "../../../assets/sinners/rodion.png";
import ryoshu from "../../../assets/sinners/ryoshu.png";
import sinclair from "../../../assets/sinners/sinclair.png";
import yiSang from "../../../assets/sinners/yi_sang.png";

/** The system plates, by the contract's id. */
export const SYSTEM_ICONS: Record<SystemName, typeof bleed> = {
  burn,
  bleed,
  tremor,
  rupture,
  sinking,
  poise,
  charge,
  slash,
  pierce,
  blunt,
};

/** The sinner portraits, by the contract's id. */
export const SINNER_PORTRAITS: Record<string, typeof yiSang> = {
  yi_sang: yiSang,
  faust,
  don_quixote: donQuixote,
  ryoshu,
  meursault,
  hong_lu: hongLu,
  heathcliff,
  ishmael,
  rodion,
  sinclair,
  outis,
  gregor,
};
