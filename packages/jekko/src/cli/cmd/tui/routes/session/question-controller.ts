import { createMemo, createSignal } from "solid-js"
import { createStore } from "solid-js/store"
import { useKeyboard } from "@opentui/solid"
import type { TextareaRenderable } from "@opentui/core"
import type { QuestionAnswer, QuestionRequest } from "@jekko-ai/sdk/v2"
import { useKeybind } from "../../context/keybind"
import { useTextareaKeybindings } from "../../component/textarea-keybindings"
import { useSDK } from "../../context/sdk"
import { useDialog } from "../../ui/dialog"

export function useQuestionPromptController(props: { request: QuestionRequest }) {
  const sdk = useSDK()
  const keybind = useKeybind()
  const bindings = useTextareaKeybindings()

  const questions = createMemo(() => props.request.questions)
  const single = createMemo(() => questions().length === 1 && questions()[0]?.multiple !== true)
  const tabs = createMemo(() => (single() ? 1 : questions().length + 1))
  const [tabHover, setTabHover] = createSignal<number | "confirm" | null>(null)
  const [store, setStore] = createStore({
    tab: 0,
    answers: [] as QuestionAnswer[],
    custom: [] as string[],
    selected: 0,
    editing: false,
  })

  let textarea: TextareaRenderable | undefined

  const question = createMemo(() => questions()[store.tab])
  const confirm = createMemo(() => !single() && store.tab === questions().length)
  const options = createMemo(() => question()?.options ?? [])
  const custom = createMemo(() => question()?.custom !== false)
  const other = createMemo(() => custom() && store.selected === options().length)
  const input = createMemo(() => store.custom[store.tab] ?? "")
  const multi = createMemo(() => question()?.multiple === true)
  const customPicked = createMemo(() => {
    const value = input()
    if (!value) return false
    return store.answers[store.tab]?.includes(value) ?? false
  })

  const textareaProps = {
    default_value: "Type your own answer",
  } as const

  function submit() {
    const answers = questions().map((_, i) => store.answers[i] ?? [])
    void sdk.client.question.reply({
      requestID: props.request.id,
      answers,
    })
  }

  function reject() {
    void sdk.client.question.reject({
      requestID: props.request.id,
    })
  }

  function pick(answer: string, customAnswer: boolean = false) {
    const answers = [...store.answers]
    answers[store.tab] = [answer]
    setStore("answers", answers)
    if (customAnswer) {
      const inputs = [...store.custom]
      inputs[store.tab] = answer
      setStore("custom", inputs)
    }
    if (single()) {
      void sdk.client.question.reply({
        requestID: props.request.id,
        answers: [[answer]],
      })
      return
    }
    setStore("tab", store.tab + 1)
    setStore("selected", 0)
  }

  function toggle(answer: string) {
    const existing = store.answers[store.tab] ?? []
    const next = [...existing]
    const index = next.indexOf(answer)
    if (index === -1) next.push(answer)
    if (index !== -1) next.splice(index, 1)
    const answers = [...store.answers]
    answers[store.tab] = next
    setStore("answers", answers)
  }

  function moveTo(index: number) {
    setStore("selected", index)
  }

  function selectTab(index: number) {
    setStore("tab", index)
    setStore("selected", 0)
  }

  function selectOption() {
    if (other()) {
      if (!multi()) {
        setStore("editing", true)
        return
      }
      const value = input()
      if (value && customPicked()) {
        toggle(value)
        return
      }
      setStore("editing", true)
      return
    }
    const opt = options()[store.selected]
    if (!opt) return
    if (multi()) {
      toggle(opt.label)
      return
    }
    pick(opt.label)
  }

  const dialog = useDialog()

  useKeyboard((evt) => {
    if (dialog.stack.length > 0) return

    if (store.editing && !confirm()) {
      if (evt.name === "escape") {
        evt.preventDefault()
        setStore("editing", false)
        return
      }
      if (keybind.match("input_clear", evt)) {
        evt.preventDefault()
        const text = textarea?.plainText ?? ""
        if (!text) {
          setStore("editing", false)
          return
        }
        textarea?.setText("")
        return
      }
      if (evt.name === "return") {
        evt.preventDefault()
        const text = textarea?.plainText?.trim() ?? ""
        const prev = store.custom[store.tab]

        if (!text) {
          if (prev) {
            const inputs = [...store.custom]
            inputs[store.tab] = ""
            setStore("custom", inputs)

            const answers = [...store.answers]
            answers[store.tab] = (answers[store.tab] ?? []).filter((x) => x !== prev)
            setStore("answers", answers)
          }
          setStore("editing", false)
          return
        }

        if (multi()) {
          const inputs = [...store.custom]
          inputs[store.tab] = text
          setStore("custom", inputs)

          const existing = store.answers[store.tab] ?? []
          const next = [...existing]
          if (prev) {
            const index = next.indexOf(prev)
            if (index !== -1) next.splice(index, 1)
          }
          if (!next.includes(text)) next.push(text)
          const answers = [...store.answers]
          answers[store.tab] = next
          setStore("answers", answers)
          setStore("editing", false)
          return
        }

        pick(text, true)
        setStore("editing", false)
        return
      }
      return
    }

    if (evt.name === "left" || evt.name === "h") {
      evt.preventDefault()
      selectTab((store.tab - 1 + tabs()) % tabs())
    }

    if (evt.name === "right" || evt.name === "l") {
      evt.preventDefault()
      selectTab((store.tab + 1) % tabs())
    }

    if (evt.name === "tab") {
      evt.preventDefault()
      const direction = evt.shift ? -1 : 1
      selectTab((store.tab + direction + tabs()) % tabs())
    }

    if (confirm()) {
      if (evt.name === "return") {
        evt.preventDefault()
        submit()
      }
      if (evt.name === "escape" || keybind.match("app_exit", evt)) {
        evt.preventDefault()
        reject()
      }
    } else {
      const opts = options()
      const total = opts.length + (custom() ? 1 : 0)
      const max = Math.min(total, 9)
      const digit = Number(evt.name)

      if (!Number.isNaN(digit) && digit >= 1 && digit <= max) {
        evt.preventDefault()
        const index = digit - 1
        moveTo(index)
        selectOption()
        return
      }

      if (evt.name === "up" || evt.name === "k") {
        evt.preventDefault()
        moveTo((store.selected - 1 + total) % total)
      }

      if (evt.name === "down" || evt.name === "j") {
        evt.preventDefault()
        moveTo((store.selected + 1) % total)
      }

      if (evt.name === "return") {
        evt.preventDefault()
        selectOption()
      }

      if (evt.name === "escape" || keybind.match("app_exit", evt)) {
        evt.preventDefault()
        reject()
      }
    }
  })

  function bindTextarea(val: TextareaRenderable) {
    textarea = val
    val.traits = { status: "ANSWER" }
    queueMicrotask(() => {
      val.focus()
      val.gotoLineEnd()
    })
  }

  return {
    questions,
    single,
    tabs,
    tabHover,
    setTabHover,
    store,
    question,
    confirm,
    options,
    custom,
    other,
    input,
    multi,
    customPicked,
    textareaProps,
    bindings,
    bindTextarea,
    selectTab,
    moveTo,
    selectOption,
  }
}

export type QuestionPromptController = ReturnType<typeof useQuestionPromptController>
