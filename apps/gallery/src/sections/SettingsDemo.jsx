import { PageHead, SettingsLayout, SettingsSection, Input, Button } from '@forge/ui';

export default function SettingsDemo() {
  return (
    <>
      <PageHead title="Settings" sub="SettingsLayout / SettingsSection / .settings-row" />
      <SettingsLayout
        nav={
          <>
            <a class="is-active" href="#settings">General</a>
            <a href="#settings">Endpoints</a>
            <a href="#settings">Tokens</a>
          </>
        }
      >
        <SettingsSection title="General" sub="Node identity and scheduling.">
          <div class="settings-row">
            <Input label="Display name" value="DGX Spark" />
            <Input label="VLAN" value="server" />
          </div>
          <div class="settings-row">
            <Input label="Model store" value="/mnt/ai-models" help="NFS mount from the NAS." />
            <Input label="Max jobs" value="4" />
          </div>
          <Button variant="primary" size="sm">Save changes</Button>
        </SettingsSection>
      </SettingsLayout>
    </>
  );
}
