// The app-drawn window chrome. `nsh.json` asks for a hidden title bar, so the
// app paints it and keeps the platform's own window buttons on the right.
import { Text, View } from "nsh";
import { isAvailable as hasWindow, titleBar } from "nsh/desktop";

/** The bar's fixed height when the platform reports none. */
const FALLBACK_HEIGHT = 40;

export function TitleBar(props: { detail: string }) {
  return (
    <If when={hasWindow}>
      <View
        desktop={{ windowDrag: true }}
        style={{
          height: titleBar().height || FALLBACK_HEIGHT,
          flexShrink: 0,
          flexDirection: "row",
          alignItems: "center",
          gap: 8,
          paddingLeft: titleBar().left + 12,
          paddingRight: titleBar().right + 12,
          backgroundColor: "#1b1114",
          borderBottomWidth: 1,
          borderBottomColor: "#462c2c",
        }}
      >
        <Text style={{ fontSize: 13, fontWeight: "600", color: "#d8d0bc" }}>AALC</Text>
        <Text style={{ fontSize: 11, color: "#b3a68a" }}>{props.detail}</Text>
      </View>
    </If>
  );
}
