import onyx from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedonyx = addPrefix(onyx, prefix);
  addBase({ ...prefixedonyx });
};
