import { useState } from 'react'
import {
  Alert,
  Button,
  Group,
  Modal,
  SegmentedControl,
  Stack,
  Text,
  Textarea,
} from '@mantine/core'
import { IconInfoCircle, IconSparkles } from '@tabler/icons-react'

interface QuickCaptureProps {
  opened: boolean
  onClose: () => void
}

/**
 * Capture shell. Milestone 3 wires "Save" to `POST /api/items`; milestone 4
 * adds AI parsing. Until then the dialog is honest about being a preview.
 */
export function QuickCapture({ opened, onClose }: QuickCaptureProps) {
  const [text, setText] = useState('')
  const [mode, setMode] = useState<'manual' | 'ai'>('manual')

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title="Quick capture"
      size="lg"
      centered
    >
      <Stack gap="md">
        <SegmentedControl
          fullWidth
          value={mode}
          onChange={(value) => setMode(value as 'manual' | 'ai')}
          data={[
            { value: 'manual', label: 'Manual' },
            { value: 'ai', label: 'AI assist' },
          ]}
        />

        <Textarea
          label="What is on your mind?"
          description="Paste a thought, a task, or a list. Nothing leaves your browser in this build."
          placeholder="Buy milk tomorrow, renew my domain on December 15, and learn Rust async programming…"
          autosize
          minRows={5}
          maxRows={12}
          value={text}
          onChange={(event) => setText(event.currentTarget.value)}
          classNames={{ input: 'prewrap' }}
        />

        {mode === 'ai' && (
          <Alert
            icon={<IconSparkles size={16} />}
            color="indigo"
            variant="light"
          >
            AI parsing is optional and arrives in milestone 4. Manual capture
            will always work without a provider.
          </Alert>
        )}

        <Alert icon={<IconInfoCircle size={16} />} variant="light" color="gray">
          <Text size="sm">
            This is the application shell. Saving items is implemented in
            milestone 3.
          </Text>
        </Alert>

        <Group justify="space-between">
          <Text size="xs" c="dimmed">
            {text.trim().length} characters
          </Text>
          <Group gap="xs">
            <Button variant="default" onClick={onClose}>
              Close
            </Button>
            <Button disabled title="Available in milestone 3">
              Save item
            </Button>
          </Group>
        </Group>
      </Stack>
    </Modal>
  )
}
