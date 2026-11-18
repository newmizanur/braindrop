import type { ReactNode } from 'react'
import { Box, Center, Group, Paper, Stack, Text, Title } from '@mantine/core'
import { Brand } from './Brand'
import { ThemeToggle } from './ThemeToggle'

interface AuthLayoutProps {
  title: string
  subtitle: string
  children: ReactNode
}

export function AuthLayout({ title, subtitle, children }: AuthLayoutProps) {
  return (
    <Box className="auth-background" mih="100dvh" pos="relative">
      <Box pos="absolute" top={16} right={16} style={{ zIndex: 1 }}>
        <ThemeToggle />
      </Box>

      <Center mih="100dvh" p="md">
        <Stack w="100%" maw={430} gap="lg">
          <Group justify="center">
            <Brand />
          </Group>

          <Paper withBorder shadow="sm" p={{ base: 'lg', sm: 'xl' }} radius="lg">
            <Stack gap="lg">
              <div>
                <Title order={2}>{title}</Title>
                <Text c="dimmed" size="sm" mt={4}>
                  {subtitle}
                </Text>
              </div>
              {children}
            </Stack>
          </Paper>

          <Text ta="center" size="xs" c="dimmed">
            Capture anything. Organize automatically.
          </Text>
        </Stack>
      </Center>
    </Box>
  )
}
