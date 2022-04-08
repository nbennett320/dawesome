import React from 'react'

interface Props {}

const TopbarBase = (props: React.PropsWithChildren<Props>) => (
  <nav className="w-full">{props.children}</nav>
)

export default TopbarBase
