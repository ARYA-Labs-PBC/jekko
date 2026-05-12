import path from "path"
import type { DialogContext } from "@tui/ui/dialog"
import * as Clipboard from "../../util/clipboard"
import * as Editor from "../../util/editor"
import { Filesystem } from "@/util/filesystem"
import { DialogExportOptions } from "../../ui/dialog-export-options"

type SessionCommandTranscriptContext = {
  session: any
  messages: any
  sync: any
  toast: any
  showThinking: any
  showDetails: any
  showAssistantMetadata: any
  renderer: any
  navigate: any
  buildSessionTranscript: (sessionData: any, sessionMessages: any[], sync: any, thinking: boolean, toolDetails: boolean, assistantMetadata: boolean) => string
  moveFirstChild: () => void
  moveChild: (direction: number) => void
  childSessionHandler: (func: (dialog: DialogContext) => void) => (dialog: DialogContext) => void
}

export function registerSessionTranscriptCommands(command: any, ctx: SessionCommandTranscriptContext) {
  const {
    session,
    messages,
    sync,
    toast,
    showThinking,
    showDetails,
    showAssistantMetadata,
    renderer,
    navigate,
    buildSessionTranscript,
    moveFirstChild,
    moveChild,
    childSessionHandler,
  } = ctx

  command.register(() => [
    {
      title: "Copy session transcript",
      value: "session.copy",
      category: "Session",
      slash: {
        name: "copy",
      },
      onSelect: async (dialog) => {
        try {
          const sessionData = session()
          if (!sessionData) return
          const sessionMessages = messages()
          const transcript = buildSessionTranscript(
            sessionData,
            sessionMessages,
            sync,
            showThinking(),
            showDetails(),
            showAssistantMetadata(),
          )
          await Clipboard.copy(transcript)
          toast.show({ message: "Session transcript copied to clipboard!", variant: "success" })
        } catch {
          toast.show({ message: "Failed to copy session transcript", variant: "error" })
        }
        dialog.clear()
      },
    },
    {
      title: "Export session transcript",
      value: "session.export",
      keybind: "session_export",
      category: "Session",
      slash: {
        name: "export",
      },
      onSelect: async (dialog) => {
        try {
          const sessionData = session()
          if (!sessionData) return
          const sessionMessages = messages()

          const defaultFilename = `session-${sessionData.id.slice(0, 8)}.md`

          const options = await DialogExportOptions.show(
            dialog,
            defaultFilename,
            showThinking(),
            showDetails(),
            showAssistantMetadata(),
            false,
          )

          if (options === null) return

          const transcript = buildSessionTranscript(
            sessionData,
            sessionMessages,
            sync,
            options.thinking,
            options.toolDetails,
            options.assistantMetadata,
          )

          if (options.openWithoutSaving) {
            // Just open in editor without saving
            await Editor.open({ value: transcript, renderer })
          } else {
            const exportDir = process.cwd()
            const filename = path.basename(options.filename.trim()) || defaultFilename
            const filepath = path.resolve(exportDir, filename)

            await Filesystem.write(filepath, transcript)

            // Open with EDITOR if available
            const result = await Editor.open({ value: transcript, renderer })
            if (result !== undefined) {
              await Filesystem.write(filepath, result)
            }

            toast.show({ message: `Session exported to ${filename}`, variant: "success" })
          }
        } catch {
          toast.show({ message: "Failed to export session", variant: "error" })
        }
        dialog.clear()
      },
    },
    {
      title: "Go to child session",
      value: "session.child.first",
      keybind: "session_child_first",
      category: "Session",
      hidden: true,
      onSelect: (dialog) => {
        moveFirstChild()
        dialog.clear()
      },
    },
    {
      title: "Go to parent session",
      value: "session.parent",
      keybind: "session_parent",
      category: "Session",
      hidden: true,
      enabled: !!session()?.parentID,
      onSelect: childSessionHandler((dialog) => {
        const parentID = session()?.parentID
        if (parentID) {
          navigate({
            type: "session",
            sessionID: parentID,
          })
          dialog.clear()
        }
      }),
    },
    {
      title: "Next child session",
      value: "session.child.next",
      keybind: "session_child_cycle",
      category: "Session",
      hidden: true,
      enabled: !!session()?.parentID,
      onSelect: childSessionHandler((dialog) => {
        moveChild(1)
        dialog.clear()
      }),
    },
    {
      title: "Previous child session",
      value: "session.child.previous",
      keybind: "session_child_cycle_reverse",
      category: "Session",
      hidden: true,
      enabled: !!session()?.parentID,
      onSelect: childSessionHandler((dialog) => {
        moveChild(-1)
        dialog.clear()
      }),
    },
  ])
}
