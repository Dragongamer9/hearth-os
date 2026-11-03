// Hearth NAME's panel. src/hearth-only.tsx must be an exact copy of hosts/decky-shared/hearth-only.tsx (copy it in when
// making the plugin; tools/decky_plugins.py check fails otherwise). Off Hearth OS the panel is the Hearth OS invitation
// and nothing else starts.
import { PanelSection, PanelSectionRow, staticClasses } from "@decky/ui";
import { definePlugin } from "@decky/api";
import { FaFire } from "react-icons/fa";
import { HearthOnly, hearthOs } from "./hearth-only";

function Panel() {
  return (
    <PanelSection>
      <PanelSectionRow>Hearth NAME</PanelSectionRow>
    </PanelSection>
  );
}

export default definePlugin(() => {
  let gone = false;
  hearthOs().then((ok) => { if (ok && !gone) { /* start what the plugin does here */ } });
  return {
    name: "Hearth NAME",
    titleView: <div className={staticClasses.Title}>Hearth NAME</div>,
    content: <HearthOnly name="Hearth NAME" what="What it does, in a few words."><Panel /></HearthOnly>,
    icon: <FaFire />,
    onDismount() { gone = true; },
  };
});
