import sorbet from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedsorbet = addPrefix(sorbet, prefix);
  addBase({ ...prefixedsorbet });
};
