import React from 'react'

interface Props {}

const PreferencePageBase = (props: React.PropsWithChildren<Props>) => (
  <div className="flex flex-col items-center justify-center">
    {props.children}
  </div>
)

export default PreferencePageBase
