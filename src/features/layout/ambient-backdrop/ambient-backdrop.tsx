import clsx from 'clsx';

// Soft colour fields behind the glass panels. Glass only reads as glass when
// there is something behind it to tint, even over a plain grey desktop.
export const AmbientBackdrop = () => (
    <div aria-hidden="true" className={clsx('pointer-events-none', 'fixed', 'inset-0', 'overflow-hidden')}>
        <div
            className={clsx(
                'absolute',
                '-top-40',
                'left-24',
                'h-[34rem]',
                'w-[34rem]',
                'rounded-full',
                'bg-sky-500/25',
                'blur-[120px]'
            )}
        />
        <div
            className={clsx(
                'absolute',
                'top-1/3',
                '-right-32',
                'h-[30rem]',
                'w-[30rem]',
                'rounded-full',
                'bg-violet-500/20',
                'blur-[120px]'
            )}
        />
        <div
            className={clsx(
                'absolute',
                '-bottom-48',
                'left-1/3',
                'h-[28rem]',
                'w-[28rem]',
                'rounded-full',
                'bg-teal-400/15',
                'blur-[120px]'
            )}
        />
    </div>
);
