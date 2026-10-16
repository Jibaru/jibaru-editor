import os from 'node:os'
import path from 'node:path'

export const FIXTURES = path.join(os.tmpdir(), 'jb-ui-fixtures')
export const fixture = (name: string) => path.join(FIXTURES, name)
