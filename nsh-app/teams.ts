// The teams page's public state: the saved teams and their list state
// (`teams-store.ts`), the editor's draft (`teams-editor.ts`), and the mirror
// model/labels (`teams-mirror.ts`). Pages keep one import path; the three
// modules can be replaced by `Teams.*` RPC calls one at a time.
export * from "./teams-store";
export * from "./teams-editor";
export {
  SYSTEM_LABELS,
  SYSTEM_NAMES,
  STARLIGHT_COSTS,
  STARLIGHT_DESCRIPTIONS,
  STARLIGHT_NAMES,
  SPIDERWEB_GIFT_ID,
  afterLevelLabel,
  discardCount,
  fixedTeamUseLabel,
  hasStarlight,
  observeGiftLabel,
  openingItemsOrderLabel,
  rewardCardsLabel,
  secondSystemFloorLabel,
  secondSystemSettingLabel,
  shoppingStrategyLabel,
  shopStrategyLabel,
  skillReplacementModeLabel,
  skillReplacementSelectLabel,
  starlightCost,
  starlightLevelLabel,
  starlightShortLevel,
  turnsLabel,
} from "./teams-mirror";
export type { DiscardSystems, SystemName, TeamMirrorConfig } from "./teams-mirror";
