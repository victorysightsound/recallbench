import rose from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedrose = addPrefix(rose, prefix);
  addBase({ ...prefixedrose });
};
