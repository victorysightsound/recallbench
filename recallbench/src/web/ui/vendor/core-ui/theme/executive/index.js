import executive from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedexecutive = addPrefix(executive, prefix);
  addBase({ ...prefixedexecutive });
};
