import sage from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedsage = addPrefix(sage, prefix);
  addBase({ ...prefixedsage });
};
