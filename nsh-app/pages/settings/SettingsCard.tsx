// The card shell every settings section uses: the active skin's header band,
// then one body with the app's shared inset and row rhythm.
import type { JSX } from "nsh";
import { Card, CardBody } from "../../components/Card";

export function SettingsCard(props: { title: string; children?: JSX.Element }) {
  return (
    <Card title={props.title}>
      <CardBody>{props.children}</CardBody>
    </Card>
  );
}
