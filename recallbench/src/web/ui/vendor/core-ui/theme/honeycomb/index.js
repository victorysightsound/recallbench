import honeycomb from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedhoneycomb = addPrefix(honeycomb, prefix);
  addBase({ ...prefixedhoneycomb });
};
