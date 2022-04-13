import React from 'react'
import Button from '../Button'
import ChevronUpDown from '../../icons/ChevronUpDown'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  value?: string | number
}

const SelectBase = (props: React.PropsWithChildren<Props<HTMLSelectElement>>) => {
  const [visible, setVisble] = React.useState(false)
  const ref = React.useRef<any>(null)

  React.useEffect(() => {
    if(ref?.current) {
      ref.current.addEventListener('keypress', (ev: KeyboardEvent): void => {
        if(ev.key === 'Escape') {
          setVisble(false)
        }
      })
    }

    return () => {
      ref.current?.removeEventListener('keypress', ref.current)
    }
  }, [ref.current])

  return (
    <div ref={ref} className="w-64">
      <div className="mt-1 relative">
        <Button
          onClick={() => setVisble(!visible)}
          onFocus={() => setVisble(true)}
          onBlur={() => setVisble(false)}
          type='button'
          style={{ width: '100%'}}
          className={`${styles.SelectBaseButton} relative bg-white rounded shadow-lg pl-3 pr-10 py-3 text-left focus:outline-none focus:ring-1 focus:ring-blue-400 focus:border-blue-400 sm:text-sm`}>
          <span className="flex items-center py-1">
            <span className="ml-3 block truncate">
              {props.value}
            </span>
          </span>
          <span className="ml-3 absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
            <ChevronUpDown />
          </span>
        </Button>

        <div
          className={`
            ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2'} absolute mt-1 w-full z-10 rounded text-sm bg-white shadow-lg transition-all ease-out duration-200`}
        >
          <ul 
            tabIndex={-1} 
            role="listbox" 
            aria-labelledby="listbox-label" 
            aria-activedescendant="listbox-item-3" 
            className="max-h-56 rounded text-base ring-1 ring-black ring-opacity-5 overflow-auto focus:outline-none sm:text-sm"
          >
            {props.children}
          </ul>
        </div>
      </div>
    </div>
  )
}

export default SelectBase
