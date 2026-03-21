import frost from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedfrost = addPrefix(frost, prefix);
  addBase({ ...prefixedfrost });
};
