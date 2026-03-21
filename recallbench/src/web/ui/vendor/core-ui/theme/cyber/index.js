import cyber from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedcyber = addPrefix(cyber, prefix);
  addBase({ ...prefixedcyber });
};
