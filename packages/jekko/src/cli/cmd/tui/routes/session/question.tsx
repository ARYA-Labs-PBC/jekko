import type { QuestionRequest } from "@jekko-ai/sdk/v2"
import { useQuestionPromptController } from "./question-controller"
import { QuestionPromptView } from "./question-view"

export function QuestionPrompt(props: { request: QuestionRequest }) {
  const controller = useQuestionPromptController(props)
  return <QuestionPromptView {...controller} />
}
