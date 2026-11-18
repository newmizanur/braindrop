import { Center, Loader, Stack, Text } from '@mantine/core'

interface FullPageLoaderProps {
  label?: string
}

export function FullPageLoader({ label = 'Loading…' }: FullPageLoaderProps) {
  return (
    <Center mih="100dvh" p="md">
      <Stack align="center" gap="sm" role="status" aria-live="polite">
        <Loader />
        <Text size="sm" c="dimmed">
          {label}
        </Text>
      </Stack>
    </Center>
  )
}
