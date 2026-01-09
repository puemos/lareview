import { Transition, Variants } from 'framer-motion';

/**
 * Tactical spring transition for micro-interactions.
 * High stiffness and damping for a snappy, premium feel with no overshoot.
 */
export const SPRING_TIGHT: Transition = {
  type: 'spring',
  stiffness: 450,
  damping: 40,
  mass: 0.8,
};

/**
 * Transition for shared layout elements (like tab backgrounds).
 */
export const SHARED_LAYOUT_TRANSITION: Transition = SPRING_TIGHT;

/**
 * Entry animation variants for list items.
 */
export const LIST_ITEM_VARIANTS: Variants = {
  hidden: { opacity: 0, y: 6, scale: 0.98 },
  visible: {
    opacity: 1,
    y: 0,
    scale: 1,
    transition: SPRING_TIGHT,
  },
};

/**
 * Tactile feedback for clickable elements.
 */
export const WHILE_TAP_SCALE = { scale: 0.985 };

/**
 * Very fast content transition (for tab switching).
 */
export const CONTENT_TRANSITION: Transition = {
  duration: 0.12,
  ease: [0.23, 1, 0.32, 1],
};
