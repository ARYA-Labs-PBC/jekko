import { DialogSelect, type DialogSelectOption } from "@tui/ui/dialog-select"
import { getZyalExample, listZyalExamples } from "@/agent-script/examples"
import { useDialog } from "@tui/ui/dialog"

export type DialogDaemonProps = {
  onSelect: (text: string) => void
}

export function buildDaemonLibraryOptions() {
  return listZyalExamples().map((example) => ({
    title: example.title,
    description: example.description,
    value: example.id,
  }))
}

export function DialogDaemon(props: DialogDaemonProps) {
  const dialog = useDialog()
  const options: DialogSelectOption<string>[] = buildDaemonLibraryOptions().map((option) => ({
    ...option,
    onSelect: () => {
      const next = getZyalExample(option.value)
      if (next) props.onSelect(next.text)
      dialog.clear()
    },
  }))

  return <DialogSelect title="Daemon library" options={options} skipFilter onSelect={() => {}} />
}
