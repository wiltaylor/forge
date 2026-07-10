import { For, Match, Show, Switch, createSignal, createUniqueId } from 'solid-js';
import { Button, Checkbox, Radio, Select } from '@forge/ui';
import type { Option } from '@forge/ui';
import type { ChatPromptControl, ChatPromptData } from './types';

/* Interactive question box. `answer` present ⇒ answered: controls disable
   (native fieldset disable) and the chosen option is highlighted. */
export interface ChatPromptProps {
  prompt: ChatPromptData;
}

export function ChatPrompt(props: ChatPromptProps) {
  const [choice, setChoice] = createSignal<string | undefined>(undefined);
  const [choices, setChoices] = createSignal<string[]>([]);
  const name = createUniqueId();

  const answered = () => props.prompt.answer !== undefined;
  const answerArr = () => {
    const a = props.prompt.answer;
    return a === undefined ? [] : Array.isArray(a) ? a : [a];
  };
  const isChosen = (v: string) => answerArr().includes(v);
  const control = () => props.prompt.control;
  const options = () => control().options;
  const selectControl = () => control() as Extract<ChatPromptControl, { type: 'select' }>;

  return (
    <fieldset class="fchat-prompt" classList={{ 'is-answered': answered() }} disabled={answered()}>
      <legend class="fchat-prompt-q">{props.prompt.question}</legend>
      <Switch>
        <Match when={control().type === 'buttons'}>
          <div class="fchat-prompt-row">
            <For each={options()}>
              {(opt: Option<string>) => (
                <Button
                  size="sm"
                  variant={answered() && isChosen(opt.value) ? 'primary' : 'secondary'}
                  disabled={opt.disabled}
                  onClick={() => props.prompt.onAnswer?.(opt.value)}
                >
                  {opt.label}
                </Button>
              )}
            </For>
          </div>
        </Match>
        <Match when={control().type === 'radio'}>
          <div class="fchat-prompt-opts" role="radiogroup">
            <For each={options()}>
              {(opt: Option<string>) => (
                <Radio
                  name={name}
                  value={opt.value}
                  disabled={opt.disabled}
                  checked={answered() ? isChosen(opt.value) : choice() === opt.value}
                  onChange={(v) => setChoice(v)}
                >
                  {opt.label}
                </Radio>
              )}
            </For>
          </div>
          <SubmitRow prompt={props.prompt} value={choice()} answered={answered()} />
        </Match>
        <Match when={control().type === 'checkbox'}>
          <div class="fchat-prompt-opts">
            <For each={options()}>
              {(opt: Option<string>) => (
                <Checkbox
                  disabled={opt.disabled}
                  checked={answered() ? isChosen(opt.value) : choices().includes(opt.value)}
                  onChange={(on) =>
                    setChoices((cur) => (on ? [...cur, opt.value] : cur.filter((v) => v !== opt.value)))
                  }
                >
                  {opt.label}
                </Checkbox>
              )}
            </For>
          </div>
          <SubmitRow
            prompt={props.prompt}
            value={choices().length ? choices() : undefined}
            answered={answered()}
          />
        </Match>
        <Match when={control().type === 'select'}>
          <Select
            options={options()}
            placeholder={selectControl().placeholder}
            value={answered() ? answerArr()[0] : choice()}
            onChange={(v) => setChoice(v)}
          />
          <SubmitRow prompt={props.prompt} value={choice()} answered={answered()} />
        </Match>
      </Switch>
      <Show when={answered()}>
        <div class="fchat-prompt-done">Answered</div>
      </Show>
    </fieldset>
  );
}

function SubmitRow(props: {
  prompt: ChatPromptData;
  value: string | string[] | undefined;
  answered: boolean;
}) {
  return (
    <Show when={!props.answered}>
      <div class="fchat-prompt-row">
        <Button
          size="sm"
          variant="primary"
          disabled={props.value === undefined}
          onClick={() => props.value !== undefined && props.prompt.onAnswer?.(props.value)}
        >
          {props.prompt.submitLabel ?? 'Submit'}
        </Button>
      </div>
    </Show>
  );
}
