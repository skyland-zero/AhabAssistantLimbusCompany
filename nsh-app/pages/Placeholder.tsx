// A page that has not been migrated yet. It says which page it is and where
// the real content comes from, instead of showing a blank region.
import { Card } from "../components/Card";
import { EmptyState } from "../components/EmptyState";
import { PageRoot } from "../components/Page";
import { pageLabel } from "../store";
import type { PageId } from "../store";

export function Placeholder(props: { page: PageId }) {
  return (
    <PageRoot>
      <Card title={pageLabel(props.page)} class="flex-1">
        <EmptyState
          title={`${pageLabel(props.page)} 页面尚未迁入`}
          detail="界面正在 P3 逐页迁移（见 MIGRATION.md）。当前这一版只包含窗口壳与设计系统组件。"
        />
      </Card>
    </PageRoot>
  );
}
