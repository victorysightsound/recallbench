import harvest from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedharvest = addPrefix(harvest, prefix);
  addBase({ ...prefixedharvest });
};
